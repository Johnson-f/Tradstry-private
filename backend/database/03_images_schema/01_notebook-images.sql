-- ============================================================================
-- Supabase Storage Bucket: notebook-images
-- ============================================================================
-- Storage path format: {user_id}/{note_id}/{image_id}.{ext}
-- 
-- Run this in Supabase SQL Editor to create the bucket and policies
-- ============================================================================

-- Create the bucket (public for direct URL access)
INSERT INTO storage.buckets (id, name, public, file_size_limit, allowed_mime_types)
VALUES (
    'notebook-images',
    'notebook-images',
    true,  -- Public bucket for direct URL access
    10485760,  -- 10MB max file size
    ARRAY['image/jpeg', 'image/png']
)
ON CONFLICT (id) DO UPDATE SET
    public = EXCLUDED.public,
    file_size_limit = EXCLUDED.file_size_limit,
    allowed_mime_types = EXCLUDED.allowed_mime_types;

-- ============================================================================
-- RLS Policies for notebook-images bucket
-- ============================================================================

-- Policy: Anyone can view images (public bucket)
CREATE POLICY "Public read access for notebook images"
ON storage.objects FOR SELECT
USING (bucket_id = 'notebook-images');

-- Policy: Authenticated users can upload to their own folder
-- Path must start with their user_id: {user_id}/...
CREATE POLICY "Users can upload images to their folder"
ON storage.objects FOR INSERT
TO authenticated
WITH CHECK (
    bucket_id = 'notebook-images'
    AND (storage.foldername(name))[1] = auth.uid()::text
);

-- Policy: Users can update their own images
CREATE POLICY "Users can update their own images"
ON storage.objects FOR UPDATE
TO authenticated
USING (
    bucket_id = 'notebook-images'
    AND (storage.foldername(name))[1] = auth.uid()::text
)
WITH CHECK (
    bucket_id = 'notebook-images'
    AND (storage.foldername(name))[1] = auth.uid()::text
);

-- Policy: Users can delete their own images
CREATE POLICY "Users can delete their own images"
ON storage.objects FOR DELETE
TO authenticated
USING (
    bucket_id = 'notebook-images'
    AND (storage.foldername(name))[1] = auth.uid()::text
);

-- Policy: Service role has full access (for backend operations)
CREATE POLICY "Service role has full access to notebook images"
ON storage.objects FOR ALL
TO service_role
USING (bucket_id = 'notebook-images')
WITH CHECK (bucket_id = 'notebook-images');
