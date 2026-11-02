class Blk < Formula
  desc "Hosts-based block list manager"
  homepage "https://github.com/OliverBrotchie/block-list"
  head "https://github.com/OliverBrotchie/block-list.git", branch: "main"
  license "GPL-3.0"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args.reject { |arg| arg == "--locked" }
  end

  test do
    assert_match "blk", shell_output("#{bin}/blk --version")
  end
end
