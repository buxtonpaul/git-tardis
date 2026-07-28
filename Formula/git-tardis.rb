class GitTardis < Formula
  desc "Time travelling git repository inspector & historical rebase tool"
  homepage "https://github.com/buxtonpaul/git-tardis"
  url "https://github.com/buxtonpaul/git-tardis/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000" # Updated on release tag
  license "MIT"
  head "https://github.com/buxtonpaul/git-tardis.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    system "#{bin}/git-tardis", "--version"
  end
end
