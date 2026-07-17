import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "Patchfleet — Local AI agent control",
  description: "A private local command center for Claude Code, Codex, and Gemini CLI.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
