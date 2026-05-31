use crate::aggregator::aggregate;
use crate::ai::{create_backend, with_retry};
use crate::cli::WatchArgs;
use crate::parser::{detect_format, parse_lines};
use crate::renderer::render_report;
use crate::types::{Level, LogEntry, Model};
use notify::{Event, RecursiveMode, Watcher};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// 读取文件的最后 N 行（用于启动时截断大文件）
fn read_last_n_lines(path: &Path, max_lines: usize) -> std::io::Result<Vec<String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let all_lines: Vec<String> = reader.lines().map_while(Result::ok).collect();

    if all_lines.len() > max_lines {
        let skipped = all_lines.len() - max_lines;
        eprintln!(
            "   文件共 {} 行，仅分析最后 {} 行（跳过 {} 行），可用 --max-initial-lines 调整",
            all_lines.len(),
            max_lines,
            skipped
        );
        Ok(all_lines.into_iter().skip(skipped).collect())
    } else {
        Ok(all_lines)
    }
}

/// 从文件指定位置读取新增的字节，返回新行和新的文件位置
fn read_new_lines(
    path: &Path,
    last_position: u64,
) -> std::io::Result<(Vec<String>, u64)> {
    let metadata = std::fs::metadata(path)?;
    let current_size = metadata.len();

    if current_size < last_position {
        // 文件被截断——返回新位置 0 让调用方处理重置
        return Ok((vec![], 0));
    }

    if current_size == last_position {
        return Ok((vec![], last_position));
    }

    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(last_position))?;

    let mut buf = String::new();
    file.read_to_string(&mut buf)?;

    let new_lines: Vec<String> = buf.lines().map(String::from).collect();
    Ok((new_lines, current_size))
}

/// 主监听循环
pub async fn watch_file(args: WatchArgs) -> anyhow::Result<()> {
    let file_path = args.file.clone();

    // ============ 启动阶段 ============

    if !file_path.exists() {
        anyhow::bail!("文件不存在: {}", file_path.display());
    }

    eprintln!("🔍 正在解析 {}...", file_path.display());

    // 确定格式
    let format = match args.format.to_format() {
        Some(f) => f,
        None => {
            let sample = read_last_n_lines(&file_path, 10)?;
            detect_format(&sample)
        }
    };

    // 读取初始内容
    let initial_lines = read_last_n_lines(&file_path, args.max_initial_lines)?;
    let initial_entries = parse_lines(&initial_lines, format);
    let min_level = args.min_level.to_level();
    let mut entries: Vec<LogEntry> = initial_entries
        .into_iter()
        .filter(|e| {
            let level = e.level.unwrap_or(Level::Unknown);
            level.severity() <= min_level.severity()
        })
        .collect();

    eprintln!("   已解析 {} 条日志", entries.len());

    // 初始 AI 分析
    let model: Model = args.model.into();
    let deep = args.deep;
    let backend = create_backend(model, deep).await?;

    let summary = aggregate(&entries);
    eprintln!(
        "   发现 {} 个错误分组, {} 个异常",
        summary.error_groups.len(),
        summary.anomalies.len()
    );

    eprintln!(
        "🤖 正在用 {} ({}) 分析...",
        backend.model_name(),
        backend.actual_model(deep)
    );
    let response = with_retry(|| backend.analyze(&summary)).await?;
    render_report(&summary, &response, 0.0, backend.model_name());

    println!(
        "\n--- 正在监听新日志 (窗口: {}秒) ---",
        args.window
    );

    // ============ 设置 notify ============

    let mut last_position = std::fs::metadata(&file_path)?.len();
    let window = args.window;
    let _start_time = Instant::now();
    let mut analysis_count: u32 = 1;
    let mut _alert_count: u32 = summary.anomalies.len() as u32;
    let mut _total_lines: u64 = entries.len() as u64;
    let mut pending_entries: Vec<LogEntry> = Vec::new();

    // notify 事件通过 std mpsc 接收，桥接到 tokio mpsc 供 select! 使用
    let (std_tx, std_rx) = mpsc::channel::<notify::Result<Event>>();
    let (async_tx, mut async_rx) = tokio::sync::mpsc::unbounded_channel::<()>();

    std::thread::spawn(move || {
        while std_rx.recv().is_ok() {
            let _ = async_tx.send(());
        }
    });

    let mut watcher = notify::recommended_watcher(move |res| {
        let _ = std_tx.send(res);
    })?;
    watcher.watch(&file_path, RecursiveMode::NonRecursive)?;

    let mut file_reappeared = false;

    // ============ 主循环 ============

    loop {
        let tick = tokio::time::sleep(Duration::from_secs(window));
        tokio::select! {
            _ = tick => {
                if !pending_entries.is_empty() {
                    let new_line_count = pending_entries.len();
                    entries.append(&mut pending_entries);
                    _total_lines += new_line_count as u64;

                    let tick_start = Instant::now();
                    let summary = aggregate(&entries);
                    let anomalies_this_tick = summary.anomalies.len() as u32;
                    _alert_count += anomalies_this_tick;

                    if anomalies_this_tick > 0 {
                        eprintln!(
                            "   ⚠️ 检测到 {} 个异常",
                            anomalies_this_tick
                        );
                    }

                    match with_retry(|| backend.analyze(&summary)).await {
                        Ok(response) => {
                            let elapsed = tick_start.elapsed().as_secs_f64();
                            analysis_count += 1;
                            println!(
                                "\n[{}] 📊 窗口 #{} · +{} 行 · 耗时 {:.1}秒",
                                chrono::Local::now().format("%H:%M:%S"),
                                analysis_count,
                                new_line_count,
                                elapsed
                            );
                            render_report(&summary, &response, elapsed, backend.model_name());
                        }
                        Err(e) => {
                            eprintln!("⚠️ AI 分析失败: {} — 跳过本次窗口", e);
                        }
                    }
                }
            }

            _ = async_rx.recv() => {
                loop {
                    match std::fs::metadata(&file_path) {
                        Ok(metadata) => {
                            let current_size = metadata.len();

                            if file_reappeared {
                                eprintln!("✅ 文件已恢复，继续监听...");
                                last_position = 0;
                                entries.clear();
                                file_reappeared = false;
                            }

                            if current_size < last_position {
                                eprintln!("⚠️ 检测到文件截断，正在重置...");
                                last_position = 0;
                                entries.clear();
                                if let Ok(restored) = read_last_n_lines(&file_path, args.max_initial_lines) {
                                    let parsed = parse_lines(&restored, format);
                                    let filtered: Vec<LogEntry> = parsed
                                        .into_iter()
                                        .filter(|e| {
                                            let level = e.level.unwrap_or(Level::Unknown);
                                            level.severity() <= min_level.severity()
                                        })
                                        .collect();
                                    entries = filtered;
                                    last_position = std::fs::metadata(&file_path).map(|m| m.len()).unwrap_or(last_position);
                                    _total_lines = entries.len() as u64;
                                }
                                break;
                            }

                            if current_size > last_position {
                                match read_new_lines(&file_path, last_position) {
                                    Ok((new_lines, new_position)) => {
                                        if !new_lines.is_empty() {
                                            let parsed = parse_lines(&new_lines, format);
                                            let filtered: Vec<LogEntry> = parsed
                                                .into_iter()
                                                .filter(|e| {
                                                    let level = e.level.unwrap_or(Level::Unknown);
                                                    level.severity() <= min_level.severity()
                                                })
                                                .collect();
                                            pending_entries.extend(filtered);
                                        }
                                        last_position = new_position;
                                    }
                                    Err(_) => {}
                                }
                            }
                            break;
                        }
                        Err(_) => {
                            eprintln!("⚠️ 文件消失，等待重新出现...");
                            loop {
                                tokio::time::sleep(Duration::from_secs(1)).await;
                                if file_path.exists() {
                                    file_reappeared = true;
                                    let _ = watcher.watch(&file_path, RecursiveMode::NonRecursive);
                                    break;
                                }
                            }
                            break;
                        }
                    }
                }
            }
        }
    }
}
