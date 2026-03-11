"use client";

import Link from "next/link";
import { Button } from "@/components/ui/button";
import { ArrowRight, Sparkles } from "lucide-react";

export function CTA() {
  return (
    <section className="py-24 px-4 sm:px-6 lg:px-8">
      <div className="container mx-auto max-w-4xl">
        <div className="relative overflow-hidden rounded-2xl border bg-gradient-to-br from-muted/50 to-muted p-12 text-center shadow-xl">
          <div 
            className="absolute inset-0 opacity-5" 
            style={{
              backgroundImage: 'radial-gradient(circle, rgba(0,0,0,0.1) 1px, transparent 1px)',
              backgroundSize: '20px 20px'
            }} 
          />
          <div className="relative z-10">
            <div className="mb-6 inline-flex items-center gap-2 rounded-full border bg-background/80 px-4 py-1.5 text-sm backdrop-blur-sm">
              <Sparkles className="size-4" />
              <span>Ready to Transform Your Trading?</span>
            </div>
            
            <h2 className="mb-4 text-3xl font-bold tracking-tight sm:text-4xl lg:text-5xl">
              Start Your Trading Journey Today
            </h2>
            
            <p className="mx-auto mb-8 max-w-2xl text-lg text-muted-foreground">
              Join traders who are already improving their performance with{" "}
              <span className="bg-gradient-to-r from-primary via-purple-600 to-pink-600 bg-clip-text text-transparent font-semibold">
                {/* Replace this with your branding */}
                YOUR-JOURNAL
              </span>
              . Get started in seconds, no credit card required.
            </p>
            
            <div className="flex flex-col items-center justify-center gap-4 sm:flex-row">
              <Link href="/auth/sign-up">
                <Button size="lg" className="group shadow-lg">
                  Create Free Account
                  <ArrowRight className="ml-2 size-4 transition-transform group-hover:translate-x-1" />
                </Button>
              </Link>
              <Link href="/auth/login">
                <Button size="lg" variant="outline">
                  Sign In
                </Button>
              </Link>
            </div>
            
            <p className="mt-6 text-sm text-muted-foreground">
              No credit card required • Cancel anytime
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}

