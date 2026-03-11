"use client";

import Link from "next/link";
import { Button } from "@/components/ui/button";
import { ArrowRight, Sparkles } from "lucide-react";

export function Hero() {
  return (
    <section className="relative flex min-h-[90vh] flex-col items-center justify-center overflow-hidden px-4 pt-12 pb-8 sm:px-6 sm:pt-16 lg:px-8">
      {/* Subtle gradient orb effects */}
      <div className="absolute top-1/4 left-1/4 -z-10 size-96 rounded-full bg-primary/10 blur-3xl" />
      <div className="absolute bottom-1/4 right-1/4 -z-10 size-96 rounded-full bg-purple-500/10 blur-3xl" />
      
      <div className="container mx-auto max-w-5xl text-center relative z-10">
        <div className="mb-6 inline-flex items-center gap-2 rounded-full border bg-muted/50 px-4 py-1.5 text-sm">
          <Sparkles className="size-4" />
          <span>AI-Powered Trading Journal</span>
        </div>
        
        <h1 className="mb-6 text-5xl font-bold tracking-tight sm:text-6xl lg:text-7xl">
          Elevate Your Trading
          <span className="block mt-2 bg-gradient-to-r from-primary via-purple-600 to-pink-600 bg-clip-text text-transparent">
            Performance
          </span>
        </h1>
        
        <div className="flex flex-col items-center justify-center gap-4 sm:flex-row">
          <Link href="/auth/sign-up">
            <Button size="lg" className="group shadow-lg">
              Get Started
              <ArrowRight className="ml-2 size-4 transition-transform group-hover:translate-x-1" />
            </Button>
          </Link>
          <Link href="/auth/login">
            <Button size="lg" variant="outline">
              Sign In
            </Button>
          </Link>
        </div>
      </div>
    </section>
  );
}

