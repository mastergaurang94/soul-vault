# Homebrew formula for Soma
# To install (once published to a tap):
#   brew install user/tap/soma
#
# To test locally:
#   brew install --build-from-source ./homebrew/soma.rb

class Soma < Formula
  desc "Your AI memory, unified — distills AI conversations into a structured local vault"
  homepage "https://github.com/user/soma"
  license "MIT"
  version "0.1.0"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/user/soma/releases/download/v#{version}/soma-macos-arm64"
      sha256 "PLACEHOLDER_SHA256_MACOS_ARM64"

      def install
        bin.install "soma-macos-arm64" => "soma"
      end
    else
      url "https://github.com/user/soma/releases/download/v#{version}/soma-macos-x86_64"
      sha256 "PLACEHOLDER_SHA256_MACOS_X86_64"

      def install
        bin.install "soma-macos-x86_64" => "soma"
      end
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/user/soma/releases/download/v#{version}/soma-linux-arm64"
      sha256 "PLACEHOLDER_SHA256_LINUX_ARM64"

      def install
        bin.install "soma-linux-arm64" => "soma"
      end
    else
      url "https://github.com/user/soma/releases/download/v#{version}/soma-linux-x86_64"
      sha256 "PLACEHOLDER_SHA256_LINUX_X86_64"

      def install
        bin.install "soma-linux-x86_64" => "soma"
      end
    end
  end

  test do
    assert_match "soma", shell_output("#{bin}/soma --help")
  end
end
