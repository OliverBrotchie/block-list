class Blk < Formula
  desc "Hosts-based block list manager"
  homepage "https://github.com/OliverBrotchie/block-list"
  url "https://github.com/OliverBrotchie/block-list/archive/refs/tags/v2.0.0.tar.gz"
  sha256 "aa9c8b54f7985b09a29cfe0afc2cef808b54f71b1074026039bccd45d732d2c0"
  license "GPL-3.0"
  head "https://github.com/OliverBrotchie/block-list.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args.reject { |arg| arg == "--locked" }
  end

  test do
    assert_match "blk", shell_output("#{bin}/blk --version")
  end
end
