import { Footer } from "@/components/footer";
import { Header } from "@/components/header";
import { ThemeProvider } from "@/components/theme-provider";
import type { Metadata } from "next";
import { Inter } from "next/font/google";
import "./globals.css";

const inter = Inter({
  variable: "--font-inter",
  subsets: ["latin"],
  display: "swap",
});

const siteUrl = "https://edgequake.com";

export const metadata: Metadata = {
  title: {
    default: "EdgeQuake — Graph-RAG Built for Speed",
    template: "%s | EdgeQuake",
  },
  description:
    "Turn documents into knowledge graphs. Query with 6 modes. 10x faster than Python RAG. Built in Rust, powered by PostgreSQL + pgvector + Apache AGE.",
  metadataBase: new URL(siteUrl),
  openGraph: {
    type: "website",
    locale: "en_US",
    url: siteUrl,
    siteName: "EdgeQuake",
    title: "EdgeQuake — Graph-RAG Built for Speed",
    description:
      "Turn documents into knowledge graphs. Query with 6 modes. 10x faster than Python RAG.",
    images: [{ url: "/og-image.png", width: 1200, height: 630, alt: "EdgeQuake" }],
  },
  twitter: {
    card: "summary_large_image",
    title: "EdgeQuake — Graph-RAG Built for Speed",
    description:
      "Turn documents into knowledge graphs. Query with 6 modes. 10x faster than Python RAG.",
    images: ["/og-image.png"],
  },
  robots: { index: true, follow: true },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="en"
      suppressHydrationWarning
      className={`${inter.variable} h-full antialiased`}
    >
      <body className="min-h-full flex flex-col">
        <ThemeProvider>
          <Header />
          <main className="flex-1">{children}</main>
          <Footer />
        </ThemeProvider>
      </body>
    </html>
  );
}
