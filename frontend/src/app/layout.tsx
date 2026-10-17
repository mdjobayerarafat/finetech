import type { Metadata } from "next";
import { Inter, Space_Grotesk, JetBrains_Mono } from "next/font/google";
import { ToastProvider } from "@/components/ui/toast";
import { AuthProvider } from "@/components/auth-provider";
import { LanguageProvider } from "@/i18n/LanguageProvider";
import { AuroraBackdrop } from "@/components/motion/aurora";
import "./globals.css";

const sans = Inter({ subsets: ["latin"], variable: "--font-sans", display: "swap" });
const display = Space_Grotesk({ subsets: ["latin"], variable: "--font-display", display: "swap" });
const mono = JetBrains_Mono({ subsets: ["latin"], variable: "--font-mono", display: "swap" });

export const metadata: Metadata = {
  title: "FinTech Pioneers — Secure AI Banking",
  description:
    "Simulation only. No real money is moved. AI may propose; software decides.",
};

export default function RootLayout({ children }: { children: React.ReactNode }) {
  return (
    <html
      lang="en"
      className={`${sans.variable} ${display.variable} ${mono.variable}`}
      suppressHydrationWarning
    >
      <body className="min-h-dvh bg-background font-sans antialiased">
        <ToastProvider>
          <LanguageProvider>
            <AuthProvider>
              <div className="relative min-h-dvh">
                <AuroraBackdrop />
                {children}
              </div>
            </AuthProvider>
          </LanguageProvider>
        </ToastProvider>
      </body>
    </html>
  );
}
