# Homebrew formula for logai
# Install with:
#   brew tap muke-muke-1/logai
#   brew install logai
#
# Or install directly from formula:
#   brew install --build-from-source ./pkg/homebrew/logai.rb

class Logai < Formula
  desc "AI-powered log analysis CLI — pipe your logs, get root cause analysis"
  homepage "https://github.com/muke-muke-1/logai"
  url "https://github.com/muke-muke-1/logai/archive/refs/tags/v0.2.0.tar.gz"
  sha256 "REPLACE_WITH_ACTUAL_SHA256"
  license "MIT"
  version "0.2.0"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    # Create a sample log file
    (testpath/"test.log").write <<~EOS
      [2026-06-01 08:03:12] ERROR ConnectionPool exhausted
      [2026-06-01 08:03:13] WARN Retry attempt 1/3
      [2026-06-01 08:03:14] ERROR timeout reading from socket
    EOS

    # Run logai analyze (won't have AI key, but parsing should succeed)
    output = shell_output("#{bin}/logai analyze #{testpath}/test.log 2>&1 || true")
    assert_match "Parsed", output
  end

  def caveats
    <<~EOS
      logai requires an AI backend API key to function. Set one of:
        export DEEPSEEK_API_KEY="sk-..."   # cheapest
        export ANTHROPIC_API_KEY="sk-ant-..."  # smartest
        export OPENAI_API_KEY="sk-..."     # enterprise

      Or use Ollama for 100% local analysis:
        ollama serve
        ollama pull llama3.2
        export OLLAMA_HOST="http://localhost:11434"

      No API key? No problem — TUI mode still works for browsing:
        logai interactive app.log
    EOS
  end
end
