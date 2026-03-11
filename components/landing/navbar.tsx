"use client";

import Link from "next/link";
import { Button } from "@/components/ui/button";
import {
  NavigationMenu,
  NavigationMenuItem,
  NavigationMenuLink,
  NavigationMenuList,
} from "@/components/ui/navigation-menu";

export function LandingNavbar() {
  return (
    <nav className="fixed top-3 left-0 right-0 z-50 flex justify-center px-3">
      <div className="w-full max-w-6xl rounded-xl border border-border/40 bg-background/85 shadow-sm backdrop-blur-md">
        <div className="flex h-12 items-center justify-between px-3 sm:px-4">
            <span
              className="text-base font-semibold tracking-tight bg-clip-text text-transparent"
              style={{
                backgroundImage:
                  "linear-gradient(to right, #FF6B35, #FF6B9D, #C44569, #6C5CE7)",
              }}
            >
              {/* Rpelace this with your journal name */}
              YOUR - JOURNAL
            </span>
          
          <div className="flex items-center gap-3">
            <NavigationMenu viewport={false} className="hidden md:flex">
              <NavigationMenuList>
                <NavigationMenuItem>
                  <Link href="/#features" legacyBehavior passHref>
                    <NavigationMenuLink className="text-sm font-medium">
                      Features
                    </NavigationMenuLink>
                  </Link>
                </NavigationMenuItem>
                <NavigationMenuItem>
                  <Link href="/#analytics" legacyBehavior passHref>
                    <NavigationMenuLink className="text-sm font-medium">
                      Analytics
                    </NavigationMenuLink>
                  </Link>
                </NavigationMenuItem>
                <NavigationMenuItem>
                  <Link href="/#pricing" legacyBehavior passHref>
                    <NavigationMenuLink className="text-sm font-medium">
                      Pricing
                    </NavigationMenuLink>
                  </Link>
                </NavigationMenuItem>
              </NavigationMenuList>
            </NavigationMenu>

            <div className="flex items-center gap-2">
            <Link href="/auth/login">
              <Button variant="ghost" size="sm">
                Log in
              </Button>
            </Link>
            <Link href="/auth/sign-up">
              <Button size="sm" className="shadow-sm">
                Get Started
              </Button>
            </Link>
            </div>
          </div>
        </div>
      </div>
    </nav>
  );
}
