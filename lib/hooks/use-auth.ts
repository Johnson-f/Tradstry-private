"use client";

import { createClient } from "@/lib/supabase/client";
import { useEffect, useState, useRef } from "react";
import { User } from "@supabase/supabase-js";
import { useRouter } from "next/navigation";

export function useAuth() {
  const [user, setUser] = useState<User | null>(null);
  const [loading, setLoading] = useState(true);
  const supabaseRef = useRef(createClient());
  const initialCheckDone = useRef(false);
  const router = useRouter();

  useEffect(() => {
    const supabase = supabaseRef.current;
    
    // CRITICAL: Check for existing session first
    const checkSession = async () => {
      try {
        const { data: { session }, error } = await supabase.auth.getSession();
        
        if (error) {
          console.error("Error checking session:", error);
          setUser(null);
        } else {
          setUser(session?.user ?? null);
        }
      } catch (error) {
        console.error("Session check failed:", error);
        setUser(null);
      } finally {
        setLoading(false);
        initialCheckDone.current = true;
      }
    };

    // Run initial session check
    checkSession();

    // Listen for auth changes (sign in, sign out, token refresh)
    const { data: { subscription } } = supabase.auth.onAuthStateChange(
      async (event, session) => {
        // Only update after initial check is done to avoid race conditions
        if (initialCheckDone.current) {
          setUser(session?.user ?? null);
          
          // Handle sign out event
          if (event === 'SIGNED_OUT') {
            setLoading(false);
            router.push('/auth/login');
          } else {
            setLoading(false);
          }
        }
      }
    );

    // Cleanup subscription on unmount
    return () => {
      subscription.unsubscribe();
    };
  }, [router]);

  return {
    user,
    loading,
    isAuthenticated: !!user,
    signOut: async () => {
      try {
        setLoading(true);
        await supabaseRef.current.auth.signOut();
        // Don't set user to null here - let onAuthStateChange handle it
        // The router.push will happen in the listener
      } catch (error) {
        console.error("Sign out error:", error);
        setLoading(false);
      }
    },
  };
}

export default useAuth;