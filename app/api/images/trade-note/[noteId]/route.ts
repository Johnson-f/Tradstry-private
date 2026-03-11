import { NextRequest, NextResponse } from 'next/server';
import { createClient } from '@/lib/supabase/server';

// Define valid sortable columns for the images table
type SortableColumn = 'created_at' | 'updated_at' | 'original_filename' | 'mime_type' | 'file_size' | 'position_in_note';

export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ noteId: string }> }
) {
  try {
    // Await the params object before accessing its properties
    const { noteId } = await params;
    
    if (!noteId) {
      return NextResponse.json({ error: 'Note ID is required' }, { status: 400 });
    }

    // Get the authenticated user
    const supabase = await createClient();
    
    const { data: { user }, error: authError } = await supabase.auth.getUser();
    
    if (authError || !user) {
      return NextResponse.json({ error: 'Unauthorized' }, { status: 401 });
    }

    // Extract query parameters for advanced filtering
    const { searchParams } = new URL(request.url);
    const includeUrls = searchParams.get('includeUrls') === 'true';
    const mimeType = searchParams.get('mimeType');
    const limit = searchParams.get('limit') ? parseInt(searchParams.get('limit')!) : undefined;
    const offset = searchParams.get('offset') ? parseInt(searchParams.get('offset')!) : 0;
    const sortByParam = searchParams.get('sortBy') || 'created_at';
    const sortOrder = searchParams.get('sortOrder') || 'desc';
    
    // Validate and cast sortBy to a valid column name
    const sortBy: SortableColumn = ['created_at', 'updated_at', 'original_filename', 'mime_type', 'file_size', 'position_in_note'].includes(sortByParam) 
      ? sortByParam as SortableColumn 
      : 'created_at';

    // Get images for the note from the database
    let query = supabase
      .from('trade-notes')
      .select('*')
      .eq('trade_note_id', noteId)
      .eq('user_id', user.id)
      .eq('is_deleted', false)
      .order(sortBy, { ascending: sortOrder === 'asc' });

    // Apply optional filters
    if (mimeType) {
      query = query.eq('mime_type', mimeType);
    }

    if (limit) {
      query = query.limit(limit);
    }

    if (offset > 0) {
      query = query.range(offset, offset + (limit || 10) - 1);
    }

    const { data: images, error: dbError } = await query;

    if (dbError) {
      console.error('Database error fetching images:', dbError);
      return NextResponse.json({ error: 'Failed to fetch images' }, { status: 500 });
    }

    if (!images || images.length === 0) {
      return NextResponse.json({
        success: true,
        message: 'No images found for this trade note',
        data: [],
        total: 0,
        trade_note_id: noteId
      });
    }

    // Generate signed URLs if requested
    let processedImages = images;
    
    if (includeUrls) {
      processedImages = await Promise.all(
        images.map(async (image) => {
          try {
            const { data: urlData, error: urlError } = await supabase.storage
              .from('trade-notes')
              .createSignedUrl(image.file_path, 3600); // 1 hour expiration

            return {
              ...image,
              signed_url: urlError ? null : urlData?.signedUrl,
              url_expires_at: urlError ? null : new Date(Date.now() + 3600 * 1000).toISOString()
            };
          } catch (error) {
            console.error(`Error creating signed URL for image ${image.id}:`, error);
            return { ...image, signed_url: null, url_expires_at: null };
          }
        })
      );
    }

    return NextResponse.json({
      success: true,
      message: `Found ${processedImages.length} images for trade note`,
      data: processedImages,
      total: processedImages.length,
      trade_note_id: noteId
    });

  } catch (error) {
    console.error('Error fetching trade note images:', error);
    return NextResponse.json(
      { error: 'Internal server error' },
      { status: 500 }
    );
  }
}

export async function POST(
  request: NextRequest,
  { params }: { params: Promise<{ noteId: string }> }
) {
  try {
    // Await the params object before accessing its properties
    const { noteId } = await params;
    
    if (!noteId) {
      return NextResponse.json({ error: 'Note ID is required' }, { status: 400 });
    }

    // Get the authenticated user
    const supabase = await createClient();
    
    const { data: { user }, error: authError } = await supabase.auth.getUser();
    
    if (authError || !user) {
      return NextResponse.json({ error: 'Unauthorized' }, { status: 401 });
    }

    const body = await request.json();
    const { operation, ...requestParams } = body;

    let result;

    switch (operation) {
      case 'bulk_get_urls':
        // Bulk generate signed URLs for images
        const { data: bulkImages, error: bulkError } = await supabase
          .from('images')
          .select('*')
          .eq('trade_note_id', noteId)
          .eq('user_id', user.id)
          .eq('is_deleted', false);

        if (bulkError) {
          throw bulkError;
        }

        result = await Promise.all(
          (bulkImages || []).map(async (image) => {
            const { data: urlData, error: urlError } = await supabase.storage
              .from('trade-notes')
              .createSignedUrl(image.file_path, requestParams.expires_in || 3600);

            return {
              ...image,
              signed_url: urlError ? null : urlData?.signedUrl,
              url_expires_at: urlError ? null : new Date(Date.now() + (requestParams.expires_in || 3600) * 1000).toISOString()
            };
          })
        );
        break;

      case 'get_by_mime_types':
        const mimeTypes = requestParams.mime_types || [];
        const { data: filteredImages, error: filterError } = await supabase
          .from('trade-notes')
          .select('*')
          .eq('trade_note_id', noteId)
          .eq('user_id', user.id)
          .eq('is_deleted', false)
          .in('mime_type', mimeTypes);

        if (filterError) {
          throw filterError;
        }

        result = filteredImages;
        break;

      case 'get_metadata_only':
        const { data: metadataImages, error: metadataError } = await supabase
          .from('trade-notes')
          .select('id, filename, mime_type, file_size, alt_text, caption, created_at, updated_at')
          .eq('trade_note_id', noteId)
          .eq('user_id', user.id)
          .eq('is_deleted', false);

        if (metadataError) {
          throw metadataError;
        }

        result = metadataImages;
        break;

      default:
        return NextResponse.json({ error: 'Invalid operation' }, { status: 400 });
    }

    return NextResponse.json({
      success: true,
      message: `Operation ${operation} completed successfully`,
      data: result,
      total: result?.length || 0,
      trade_note_id: noteId
    });

  } catch (error) {
    console.error('Error in POST trade note images:', error);
    return NextResponse.json(
      { error: 'Internal server error' },
      { status: 500 }
    );
  }
}
