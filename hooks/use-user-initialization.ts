'use client';

import { useEffect, useState, useCallback, useRef } from 'react';
import { createClient } from '@/lib/supabase/client';
import { initializeUser, checkUserInitialization, clearUserCheckCache } from '@/lib/services/user-service';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { toast } from 'sonner';

// Global flag to prevent multiple simultaneous initialization attempts
let globalInitializationInProgress = false;

interface UserInitializationState {
  isInitialized: boolean;
  isInitializing: boolean;
  error: string | null;
  needsRefresh: boolean;
}

// Use localStorage instead of sessionStorage to persist across refreshes
const STORAGE_KEY_PREFIX = 'user-init-status-';

/**
 * Hook to handle user initialization after login
 * This runs once when the user first accesses the protected app
 * Implements retry logic: tries twice, then prompts user to refresh
 */
export function useUserInitialization() {
  const [state, setState] = useState<UserInitializationState>({
    isInitialized: false,
    isInitializing: false,
    error: null,
    needsRefresh: false,
  });

  const queryClient = useQueryClient();
  const hasInitializedRef = useRef(false); // Prevent double-initialization in strict mode
  
  const initializeMutation = useMutation({
    mutationKey: ['user', 'initialize'],
    mutationFn: async ({ email, userId }: { email: string; userId: string }) => {
      return await initializeUser(email, userId, 2, 1000);
    },
    gcTime: Infinity,
    retry: false,
  });

  const handleRefreshPrompt = useCallback(() => {
    toast.error(
      'Unable to initialize your account. Please refresh the page to try again.',
      {
        duration: 10000,
        action: {
          label: 'Refresh Page',
          onClick: () => window.location.reload(),
        },
      }
    );
  }, []);

  useEffect(() => {
    let mounted = true;
    const supabase = createClient();

    const initializeUserOnLogin = async () => {
      // Prevent double initialization in React strict mode
      if (hasInitializedRef.current) {
        return;
      }

      try {
        // Get current user session
        const { data: { user }, error: userError } = await supabase.auth.getUser();
        
        if (userError || !user) {
          // No user yet; wait for auth state change
          return;
        }

        // Check localStorage for initialization status
        const initKey = `${STORAGE_KEY_PREFIX}${user.id}`;
        const storedStatus = localStorage.getItem(initKey);
        
        // Verify with backend if localStorage says success
        if (storedStatus === 'success') {
          try {
            // Verify with backend to ensure consistency (uses session cache)
            const checkResult = await checkUserInitialization(user.id);
            
            if (checkResult.exists) {
              // Backend confirms initialization - good to go
              hasInitializedRef.current = true;
              if (mounted) {
                setState({
                  isInitialized: true,
                  isInitializing: false,
                  error: null,
                  needsRefresh: false,
                });
              }
              return;
            } else {
              // Backend says not initialized but localStorage says success
              // Clear localStorage and proceed with initialization flow
              localStorage.removeItem(initKey);
              clearUserCheckCache(user.id);
            }
          } catch (error) {
            console.error('Error verifying user initialization:', error);
            // Fall back to localStorage if backend check fails
            hasInitializedRef.current = true;
            if (mounted) {
              setState({
                isInitialized: true,
                isInitializing: false,
                error: null,
                needsRefresh: false,
              });
            }
            return;
          }
        }

        // If localStorage says failed, try backend check as last resort
        if (storedStatus === 'failed') {
          try {
            const checkResult = await checkUserInitialization(user.id, true); // Skip cache for failed status
            
            if (checkResult.exists) {
              // Backend says user IS initialized - clear failed status
              localStorage.setItem(initKey, 'success');
              queryClient.setQueryData(['user', 'initialized', user.id], true);
              hasInitializedRef.current = true;
              if (mounted) {
                setState({
                  isInitialized: true,
                  isInitializing: false,
                  error: null,
                  needsRefresh: false,
                });
              }
              return;
            }
          } catch (error) {
            console.error('Error checking user initialization:', error);
            // Continue with failed status if backend check fails
          }
          
          // Backend confirms user is not initialized
          hasInitializedRef.current = true;
          if (mounted) {
            setState({
              isInitialized: false,
              isInitializing: false,
              error: 'Initialization failed. Please refresh the page.',
              needsRefresh: true,
            });
            handleRefreshPrompt();
          }
          return;
        }

        // Check if another initialization is already in progress globally
        if (globalInitializationInProgress) {
          return;
        }

        // Mark as initialized to prevent re-runs
        hasInitializedRef.current = true;

        // Set global flag and local state
        globalInitializationInProgress = true;
        if (mounted) {
          setState(prev => ({
            ...prev,
            isInitializing: true,
            error: null,
            needsRefresh: false,
          }));
        }

        // Check cache first
        const cached = queryClient.getQueryData<boolean>(['user', 'initialized', user.id]);
        if (cached) {
          localStorage.setItem(initKey, 'success');
          if (mounted) {
            setState({
              isInitialized: true,
              isInitializing: false,
              error: null,
              needsRefresh: false,
            });
          }
          globalInitializationInProgress = false;
          return;
        }

        // Check backend before attempting initialization
        try {
          const checkResult = await checkUserInitialization(user.id);
          if (checkResult.exists) {
            // User is already initialized on backend
            localStorage.setItem(initKey, 'success');
            queryClient.setQueryData(['user', 'initialized', user.id], true);
            
            if (mounted) {
              setState({
                isInitialized: true,
                isInitializing: false,
                error: null,
                needsRefresh: false,
              });
            }
            globalInitializationInProgress = false;
            return;
          }
        } catch (error) {
          console.error('Error checking user initialization before init attempt:', error);
          // Continue with initialization if check fails
        }

        // Attempt initialization with retry logic built into the service
        const initResult = await initializeMutation.mutateAsync({ 
          email: user.email!, 
          userId: user.id 
        });
        
        if (mounted) {
          // Explicitly check for boolean true
          if (initResult.success === true) {
            localStorage.setItem(initKey, 'success');
            queryClient.setQueryData(['user', 'initialized', user.id], true);
            clearUserCheckCache(user.id); // Clear cache so next check gets fresh data
            
            setState({
              isInitialized: true,
              isInitializing: false,
              error: null,
              needsRefresh: false,
            });
            
            toast.success('Account initialized successfully!');
          } else {
            localStorage.setItem(initKey, 'failed');
            
            setState({
              isInitialized: false,
              isInitializing: false,
              error: initResult.message || 'Initialization failed after multiple attempts',
              needsRefresh: true,
            });
            
            handleRefreshPrompt();
          }
        }
        
        globalInitializationInProgress = false;
      } catch (error) {
        if (mounted) {
          const errorMessage = error instanceof Error ? error.message : 'Unknown initialization error';
          
          const supabase = createClient();
          const { data: { user } } = await supabase.auth.getUser();
          if (user) {
            const initKey = `${STORAGE_KEY_PREFIX}${user.id}`;
            localStorage.setItem(initKey, 'failed');
          }
          
          setState({
            isInitialized: false,
            isInitializing: false,
            error: errorMessage,
            needsRefresh: true,
          });
          
          handleRefreshPrompt();
        }
        
        globalInitializationInProgress = false;
      }
    };

    initializeUserOnLogin();

    // If no user at first, listen for auth state changes to trigger initialization later
    const { data: authListener } = supabase.auth.onAuthStateChange((event, session) => {
      if (!mounted) return;
      if (hasInitializedRef.current) return;
      if (session?.user) {
        initializeUserOnLogin();
      }
    });

    return () => {
      mounted = false;
      authListener.subscription.unsubscribe();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []); // Empty dependency array - run only once on mount

  return state;
}

// Export utility function to clear initialization status (for logout)
export function clearUserInitializationStatus(userId: string) {
  localStorage.removeItem(`${STORAGE_KEY_PREFIX}${userId}`);
}