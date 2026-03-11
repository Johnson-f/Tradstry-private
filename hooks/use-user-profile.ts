import { useEffect, useState } from 'react';
import { createClient } from '@/lib/supabase/client';

export interface UserProfileData {
  nickname: string | null;
  display_name: string | null;
  timezone: string | null;
  currency: string | null;
  trading_experience_level: string | null;
  primary_trading_goal: string | null;
  asset_types: string | null;
  trading_style: string | null;
  profile_picture_uuid: string | null;
}

type UserProfile = {
  firstName: string;
  displayName: string;
  email: string;
  profileData: UserProfileData | null;
  loading: boolean;
  error: Error | null;
  refetch: () => Promise<void>;
};

export function useUserProfile(): UserProfile {
  const [profile, setProfile] = useState<UserProfile>({
    firstName: '',
    displayName: '',
    email: '',
    profileData: null,
    loading: true,
    error: null,
    refetch: async () => {},
  });

  const fetchUserProfile = async () => {
    try {
      setProfile(prev => ({ ...prev, loading: true, error: null }));
      
      const supabase = createClient();
      const { data: { user }, error: authError } = await supabase.auth.getUser();
      
      if (!user || authError) {
        setProfile(prev => ({
          ...prev,
          loading: false,
          error: authError || new Error('No authenticated user')
        }));
        return;
      }

      const userEmail = user.email || '';
      
      // Fetch profile from backend
      try {
        const response = await fetch(`/api/user/profile/${user.id}`, {
          method: 'GET',
          headers: { 'Content-Type': 'application/json' },
        });

        if (response.ok) {
          const result = await response.json();
          const profileData = result.profile as UserProfileData;
          
          // Determine display name and first name
          const displayName = profileData?.nickname || profileData?.display_name || userEmail.split('@')[0];
          const firstName = profileData?.nickname?.split(' ')[0] || 
                          profileData?.display_name?.split(' ')[0] || 
                          userEmail.split('@')[0];
          
          setProfile({
            firstName,
            displayName,
            email: userEmail,
            profileData,
            loading: false,
            error: null,
            refetch: fetchUserProfile,
          });
        } else {
          // Profile doesn't exist yet - use email fallback
          const emailUsername = userEmail.split('@')[0];
          const firstName = emailUsername.charAt(0).toUpperCase() + emailUsername.slice(1);
          
          setProfile({
            firstName,
            displayName: firstName,
            email: userEmail,
            profileData: null,
            loading: false,
            error: null,
            refetch: fetchUserProfile,
          });
        }
      } catch (fetchError) {
        // Fallback to email-based name on fetch error
        const emailUsername = userEmail.split('@')[0];
        const firstName = emailUsername.charAt(0).toUpperCase() + emailUsername.slice(1);
        
        setProfile({
          firstName,
          displayName: firstName,
          email: userEmail,
          profileData: null,
          loading: false,
          error: fetchError instanceof Error ? fetchError : new Error('Failed to fetch profile'),
          refetch: fetchUserProfile,
        });
      }
    } catch (err) {
      setProfile(prev => ({
        ...prev,
        loading: false,
        error: err instanceof Error ? err : new Error('Failed to load user data')
      }));
    }
  };

  useEffect(() => {
    fetchUserProfile();
  }, []);

  return profile;
}