import type { Metadata, Viewport } from "next";
import { Instrument_Sans } from "next/font/google";
import { SiteFooter } from "@/components/site-footer";
import { SiteNav } from "@/components/site-nav";
import { ThemeProvider } from "@/components/theme-provider";
import "./globals.css";

const instrument = Instrument_Sans({
  subsets: ["latin"],
  variable: "--font-instrument",
  display: "swap",
  weight: ["400", "500", "600", "700"],
});

const siteUrl =
  process.env.NEXT_PUBLIC_SITE_URL ?? "https://cryptoduke01.github.io/onca-site";

export const metadata: Metadata = {
  title: {
    default: "Onca · Solana tools for ZeroClaw agents",
    template: "%s · Onca",
  },
  description:
    "Onca gives a ZeroClaw agent safe hands on Solana. The agent proposes, a human disposes. Read-only and build-only tools. No spendable keys.",
  metadataBase: new URL(siteUrl),
  openGraph: {
    title: "Onca · Solana tools for ZeroClaw agents",
    description:
      "Safe Solana tools for ZeroClaw. T0 reads and T1 unsigned builds. No spendable keys.",
    type: "website",
  },
  icons: {
    icon: [
      { url: "/favicon.ico", sizes: "any" },
      { url: "/favicon.svg", type: "image/svg+xml" },
      { url: "/icon-32.png", type: "image/png", sizes: "32x32" },
    ],
    apple: [{ url: "/apple-touch-icon.png", sizes: "180x180" }],
  },
};

export const viewport: Viewport = {
  themeColor: [
    { media: "(prefers-color-scheme: dark)", color: "#050506" },
    { media: "(prefers-color-scheme: light)", color: "#f4f5f7" },
  ],
  width: "device-width",
  initialScale: 1,
};

const themeBoot = `(function(){try{var t=localStorage.getItem('onca-theme');if(t==='light'||(t!=='dark'&&matchMedia('(prefers-color-scheme: light)').matches))document.documentElement.classList.add('light')}catch(e){}})();`;

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" className={instrument.variable} suppressHydrationWarning>
      <head>
        <script dangerouslySetInnerHTML={{ __html: themeBoot }} />
      </head>
      <body className="min-h-dvh font-sans antialiased">
        <ThemeProvider>
          <SiteNav />
          {children}
          <SiteFooter />
        </ThemeProvider>
      </body>
    </html>
  );
}
