import type { Metadata } from "next";
import { Geist } from "next/font/google";
import { AuthWrapper } from "@/components/auth-wrapper";
import "./globals.css";
import React from "react";
 

const defaultUrl = process.env.VERCEL_URL
  ? `https://${process.env.VERCEL_URL}`
  // Replace this with your domain if you want to use via https 
  : "https://your-domain.com";

export const metadata: Metadata = {
  metadataBase: new URL(defaultUrl),
  title: "Tradstry",
  description: "An AI-powered trading journal for traders and investors",
  icons: {
    icon: [
      {
        url: '/icon.png',
        sizes: '32x32',
        type: 'image/png',
      },
      {
        url: '/icon.png',
        sizes: '64x64',
        type: 'image/png',
      },
      {
        url: '/icon.png',
        sizes: '128x128',
        type: 'image/png',
      },
      {
        url: '/favicon.ico',
        sizes: '48x48',
        type: 'image/x-icon',
      },
    ],
    shortcut: '/favicon.ico',
    apple: [
      {
        url: '/icon.png',
        sizes: '180x180',
        type: 'image/png',
      },
    ],
  },
};

const geistSans = Geist({
  variable: "--font-geist-sans",
  display: "swap",
  subsets: ["latin"],
});

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body className={`${geistSans.className} antialiased`}>
        <AuthWrapper>{children}</AuthWrapper>
      </body>
    </html>
  );
}