import { NextRequest, NextResponse } from 'next/server';
import { createClient } from '@/lib/supabase/server';

export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ imageId: string }> }
) {
  try {
    // Await the params object before accessing its properties
    const { imageId } = await params;
    
    if (!imageId) {
      return NextResponse.json({ error: 'Image ID is required' }, { status: 400 });
    }

    // Get the authenticated user
    const supabase = await createClient();
    
    const { data: { user }, error: authError } = await supabase.auth.getUser();
    
    if (authError || !user) {
      return NextResponse.json({ error: 'Unauthorized' }, { status: 401 });
    }

    // Get image metadata from database
    const { data: imageData, error: imageError } = await supabase
      .rpc('get_image_by_id', {
        p_image_id: imageId
      });

    if (imageError || !imageData || imageData.length === 0) {
      return NextResponse.json({ error: 'Image not found' }, { status: 404 });
    }

    const image = imageData[0];
    
    // Get signed URL from Supabase Storage
    const { data: urlData, error: urlError } = await supabase.storage
      .from('notebook')
      .createSignedUrl(image.file_path, 3600); // 1 hour expiration

    if (urlError || !urlData?.signedUrl) {
      return NextResponse.json({ error: 'Failed to generate image URL' }, { status: 500 });
    }

    // Fetch the actual image data
    const imageResponse = await fetch(urlData.signedUrl);
    
    if (!imageResponse.ok) {
      return NextResponse.json({ error: 'Failed to fetch image' }, { status: 500 });
    }

    const imageBuffer = await imageResponse.arrayBuffer();
    
    // Return the image with proper headers
    return new NextResponse(imageBuffer, {
      status: 200,
      headers: {
        'Content-Type': image.mime_type || 'image/jpeg',
        'Content-Length': image.file_size?.toString() || '',
        'Cache-Control': 'public, max-age=3600, s-maxage=3600',
        'ETag': `"${imageId}"`,
        'Last-Modified': new Date(image.updated_at || image.created_at).toUTCString(),
      },
    });

  } catch (error) {
    console.error('Error serving image:', error);
    return NextResponse.json(
      { error: 'Internal server error' },
      { status: 500 }
    );
  }
}