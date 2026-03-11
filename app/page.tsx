import type { Metadata } from "next";
import {
  LandingNavbar,
  Hero,
  Features,
  CTA,
  Footer,
} from "@/components/landing";

// Force this page to be static - because of SEO
export const dynamic = 'force-static';
export const revalidate = 3600; // Revalidate every hour (optional)

const defaultUrl = process.env.VERCEL_URL
  ? `https://${process.env.VERCEL_URL}`
  : "https://tradstry.com";

export const metadata: Metadata = {
  metadataBase: new URL(defaultUrl),
  title: "Tradstry - AI-Powered Trading Journal & Analytics Platform",
  description:
    "Track, analyze, and improve your trading performance with comprehensive journaling, real-time analytics, and AI-powered insights. Transform your trading journey with data-driven decisions.",
  keywords: [
    "trading journal",
    "trading analytics",
    "trade tracking",
    "trading platform",
    "stock trading",
    "trading performance",
    "AI trading insights",
    "trading journal software",
    "portfolio analytics",
    "trading education",
  ],
  authors: [{ name: "Tradstry" }],
  creator: "Tradstry",
  openGraph: {
    type: "website",
    locale: "en_US",
    url: defaultUrl,
    title: "Tradstry - AI-Powered Trading Journal & Analytics Platform",
    description:
      "Track, analyze, and improve your trading performance with comprehensive journaling, real-time analytics, and AI-powered insights.",
    siteName: "Tradstry",
  },
  twitter: {
    card: "summary_large_image",
    title: "Tradstry - AI-Powered Trading Journal & Analytics Platform",
    description:
      "Track, analyze, and improve your trading performance with comprehensive journaling, real-time analytics, and AI-powered insights.",
    creator: "@tradstry",
  },
  robots: {
    index: true,
    follow: true,
    googleBot: {
      index: true,
      follow: true,
      "max-video-preview": -1,
      "max-image-preview": "large",
      "max-snippet": -1,
    },
  },
  alternates: {
    canonical: defaultUrl,
  },
};

export default function Home() {
  return (
    <div className="relative flex min-h-screen flex-col">
      {/* Icon-inspired gradient background */}
      <div className="fixed inset-0 -z-10 bg-[radial-gradient(circle_at_center,#ff6b35_0%,#ff6b9d_28%,#c44569_52%,#6c5ce7_78%,#050505_100%)]" />
      <div className="fixed inset-0 -z-10 bg-[radial-gradient(circle_at_20%_25%,rgba(255,107,53,0.18),transparent_45%),radial-gradient(circle_at_80%_75%,rgba(108,92,231,0.16),transparent_55%)] mix-blend-screen opacity-80" />
      
      <LandingNavbar />
      <main className="flex-1 relative">
        <Hero />
        <div id="features">
          <Features />
        </div>
        <CTA />
      </main>
      <Footer />
    </div>
  );
}
