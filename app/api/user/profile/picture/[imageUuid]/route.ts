import { NextRequest, NextResponse } from 'next/server';
import { createClient } from '@/lib/supabase/server';

export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ imageUuid: string }> }
) {
  try {
    const { imageUuid } = await params;

    if (!imageUuid) {
      return NextResponse.json({ error: 'Image UUID is required' }, { status: 400 });
    }

    // Get the authenticated user
    const supabase = await createClient();
    const { data: { user }, error: authError } = await supabase.auth.getUser();

    if (authError || !user) {
      return NextResponse.json({ error: 'Unauthorized' }, { status: 401 });
    }

    // Get image metadata from Supabase user_profile_images table
    const { data: imageData, error: imageError } = await supabase
      .from('user_profile_images')
      .select('file_path, mime_type, file_size, user_id')
      .eq('image_uuid', imageUuid)
      .single();

    if (imageError || !imageData) {
      return NextResponse.json({ error: 'Image not found' }, { status: 404 });
    }

    // Verify the image belongs to the requesting user (or allow public access if needed)
    // For now, we'll only allow users to access their own images
    if (imageData.user_id !== user.id) {
      return NextResponse.json({ error: 'Unauthorized' }, { status: 403 });
    }

    // Get signed URL from Supabase Storage using the stored file_path
    const { data: urlData, error: urlError } = await supabase.storage
      .from('profile-pictures')
      .createSignedUrl(imageData.file_path, 3600); // 1 hour expiration

    if (urlError || !urlData?.signedUrl) {
      return NextResponse.json({ error: 'Failed to generate image URL' }, { status: 500 });
    }

    // Fetch the actual image data
    const imageResponse = await fetch(urlData.signedUrl);
    
    if (!imageResponse.ok) {
      return NextResponse.json({ error: 'Failed to fetch image' }, { status: 500 });
    }

    const imageBuffer = await imageResponse.arrayBuffer();
    const contentType = imageData.mime_type || 'image/jpeg';

    // Return the image with proper headers
    return new NextResponse(imageBuffer, {
      status: 200,
      headers: {
        'Content-Type': contentType,
        'Cache-Control': 'public, max-age=3600, s-maxage=3600',
        'ETag': `"${imageUuid}"`,
      },
    });

  } catch (error) {
    console.error('Error serving profile image:', error);
    return NextResponse.json(
      { error: 'Internal server error' },
      { status: 500 }
    );
  }
}

