import type { Metadata } from "next";
import { JetBrains_Mono } from "next/font/google";
import "./globals.css";
import { ClientLayout } from "./client-layout";

const jetbrainsMono = JetBrains_Mono({subsets:['latin'],variable:'--font-sans'});

export const metadata: Metadata = {
  title: "LLM Gateway Admin",
  description: "Admin console for LLM Gateway",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className={jetbrainsMono.variable}>
      <body className="antialiased font-sans">
        <ClientLayout>{children}</ClientLayout>
      </body>
    </html>
  );
}
