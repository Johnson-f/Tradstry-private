"use client";

import Link from 'next/link';

export default function NotFound() {
  return (
    <div className="min-h-screen flex items-center justify-center">
      <div className="text-center">
        <h2 className="text-2xl font-bold mb-2">404 - Page Not Found</h2>
        <p className="text-gray-600 mb-4">The page you&apos;re looking for doesn&apos;t exist.</p>
        <Link href="/" className="text-blue-600 hover:underline">
          Return to home
        </Link>
      </div>
    </div>
  );
}