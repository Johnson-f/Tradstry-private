import { NextRequest, NextResponse } from 'next/server';
import { createClient } from '@/lib/supabase/server';
import { apiConfig, getFullUrl } from '@/lib/config/api';

export async function POST(request: NextRequest) {
  const startTime = Date.now();
  console.log('[POST /api/user/profile/picture] Starting request');
  
  try {
    console.log('[POST /api/user/profile/picture] Step 1: Creating Supabase client');
    const supabase = await createClient();
    
    console.log('[POST /api/user/profile/picture] Step 2: Getting user from auth');
    const { data: { user }, error: authError } = await supabase.auth.getUser();

    if (authError || !user) {
      console.error('[POST /api/user/profile/picture] Auth error:', {
        authError: authError?.message,
        hasUser: !!user,
        userId: user?.id,
      });
      return NextResponse.json({ error: 'Unauthorized' }, { status: 401 });
    }
    
    console.log('[POST /api/user/profile/picture] Step 3: User authenticated', {
      userId: user.id,
      email: user.email,
    });

    // Get auth token for backend request
    console.log('[POST /api/user/profile/picture] Step 4: Getting session');
    const { data: { session }, error: sessionError } = await supabase.auth.getSession();
    
    if (sessionError) {
      console.error('[POST /api/user/profile/picture] Session error:', sessionError);
      return NextResponse.json({ error: 'Session error' }, { status: 401 });
    }
    
    if (!session?.access_token) {
      console.error('[POST /api/user/profile/picture] No access token in session');
      return NextResponse.json({ error: 'No session' }, { status: 401 });
    }
    
    console.log('[POST /api/user/profile/picture] Step 5: Session valid, token length:', session.access_token.length);

    console.log('[POST /api/user/profile/picture] Step 6: Parsing form data');
    const formData = await request.formData();
    const file = formData.get('file') as File;

    if (!file) {
      console.error('[POST /api/user/profile/picture] No file in form data');
      const formDataKeys = Array.from(formData.keys());
      console.error('[POST /api/user/profile/picture] FormData keys:', formDataKeys);
      return NextResponse.json({ error: 'No file provided' }, { status: 400 });
    }

    console.log('[POST /api/user/profile/picture] Step 7: File received', {
      fileName: file.name,
      fileSize: file.size,
      fileType: file.type,
    });

    // Check file size (5MB max)
    const MAX_FILE_SIZE = 5 * 1024 * 1024;
    if (file.size > MAX_FILE_SIZE) {
      console.error('[POST /api/user/profile/picture] File too large', {
        fileSize: file.size,
        maxSize: MAX_FILE_SIZE,
      });
      return NextResponse.json(
        { error: 'File size exceeds 5MB limit' },
        { status: 400 }
      );
    }

    // Check file type
    if (!file.type.startsWith('image/')) {
      console.error('[POST /api/user/profile/picture] Invalid file type', {
        fileType: file.type,
      });
      return NextResponse.json(
        { error: 'Only image files are allowed' },
        { status: 400 }
      );
    }

    console.log('[POST /api/user/profile/picture] Step 8: File validation passed');

    // Create FormData for backend
    console.log('[POST /api/user/profile/picture] Step 9: Creating FormData for backend');
    const backendFormData = new FormData();
    backendFormData.append('file', file);

    // Call backend API to upload profile picture
    const backendUrl = getFullUrl(apiConfig.endpoints.user.profilePicture(user.id));
    console.log('[POST /api/user/profile/picture] Step 10: Calling backend API', {
      backendUrl,
      userId: user.id,
      hasToken: !!session.access_token,
    });

    try {
      const response = await fetch(backendUrl, {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${session.access_token}`,
        },
        body: backendFormData,
      });

      console.log('[POST /api/user/profile/picture] Step 11: Backend response received', {
        status: response.status,
        statusText: response.statusText,
        ok: response.ok,
        headers: Object.fromEntries(response.headers.entries()),
      });

      if (!response.ok) {
        const errorText = await response.text();
        console.error('[POST /api/user/profile/picture] Backend error response', {
          status: response.status,
          statusText: response.statusText,
          errorText,
        });
        return NextResponse.json(
          { error: errorText || 'Failed to upload profile picture' },
          { status: response.status }
        );
      }

      console.log('[POST /api/user/profile/picture] Step 12: Parsing backend JSON response');
      const result = await response.json();
      console.log('[POST /api/user/profile/picture] Backend result:', {
        success: result.success,
        hasImageUuid: !!result.image_uuid,
        imageUuid: result.image_uuid,
        filePath: result.file_path,
        fileSize: result.file_size,
        mimeType: result.mime_type,
        originalFilename: result.original_filename,
      });

      if (!result.success || !result.image_uuid) {
        console.error('[POST /api/user/profile/picture] Invalid backend response', {
          result,
        });
        return NextResponse.json(
          { error: 'Upload failed or invalid response' },
          { status: 500 }
        );
      }

      console.log('[POST /api/user/profile/picture] Step 13: Storing image metadata in Supabase');
      // Store image metadata in Supabase user_profile_images table
      const { data: insertData, error: dbError } = await supabase
        .from('user_profile_images')
        .insert({
          user_id: user.id,
          image_uuid: result.image_uuid,
          file_path: result.file_path,
          file_size: result.file_size,
          mime_type: result.mime_type,
          original_filename: result.original_filename,
          bucket_name: 'profile-pictures',
        })
        .select();

      if (dbError) {
        console.error('[POST /api/user/profile/picture] Failed to store image metadata:', {
          error: dbError.message,
          code: dbError.code,
          details: dbError.details,
          hint: dbError.hint,
          insertData,
        });
        // Don't fail the request - file is already uploaded
        // Just log the error
      } else {
        console.log('[POST /api/user/profile/picture] Step 14: Metadata stored successfully', {
          insertData,
        });
      }

      const duration = Date.now() - startTime;
      console.log('[POST /api/user/profile/picture] Success!', {
        imageUuid: result.image_uuid,
        duration: `${duration}ms`,
      });

      return NextResponse.json({
        success: true,
        image_uuid: result.image_uuid,
        message: 'Profile picture uploaded successfully',
      });
    } catch (fetchError) {
      console.error('[POST /api/user/profile/picture] Fetch error:', {
        error: fetchError instanceof Error ? fetchError.message : String(fetchError),
        stack: fetchError instanceof Error ? fetchError.stack : undefined,
        backendUrl,
      });
      throw fetchError;
    }
  } catch (error) {
    const duration = Date.now() - startTime;
    console.error('[POST /api/user/profile/picture] Caught error:', {
      error: error instanceof Error ? error.message : String(error),
      stack: error instanceof Error ? error.stack : undefined,
      name: error instanceof Error ? error.name : undefined,
      duration: `${duration}ms`,
    });
    return NextResponse.json(
      { 
        error: 'Internal server error',
        message: error instanceof Error ? error.message : String(error),
      },
      { status: 500 }
    );
  }
}

