cask "scenic-motion-engine" do
  version :latest
  # TODO: paste real sha256 after first release
  sha256 :no_check

  url "https://github.com/danielesiegel/scenic-motion-engine-mac/releases/latest/download/scenic-motion-engine.dmg"
  name "Scenic Motion Engine"
  desc "Motion engine for scenic rendering on macOS"
  homepage "https://github.com/danielesiegel/scenic-motion-engine-mac"

  app "Scenic Motion Engine.app"

  zap trash: [
    "~/Library/Application Support/Scenic Motion Engine",
    "~/Library/Caches/com.danielesiegel.scenic-motion-engine",
    "~/Library/Preferences/com.danielesiegel.scenic-motion-engine.plist",
    "~/Library/Logs/Scenic Motion Engine",
  ]
end
