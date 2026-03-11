<!-- bc25ba1c-a839-4883-87f8-525376eb73f4 1ab6633b-a333-4628-a727-84406df39e2c -->
# Notebook Image Upload Integration Plan

## Overview

Integrate image uploads into the BlockNote editor using Supabase Storage bucket, with metadata stored in a Supabase table, user-specific folder structure, private access control, and automatic cleanup on note deletion.

## Architecture Decisions

- **Storage**: Supabase Storage bucket named `notebook-images`
- **Folder Structure**: `{userId}/notebooks/{noteId}/{imageId}.ext`
- **Access Control**: Private (only authenticated user can access)
- **Metadata Storage**: Supabase table (not Turso user databases)
- **Lifecycle**: Delete images immediately from storage when note is deleted
- **Editor Integration**: Use BlockNote's `uploadFile` option

## Implementation Steps

### 1. Supabase Setup

**Create Storage Bucket:**

- Bucket name: `notebook-images`
- Access: Private
- File size limit: 5MB (configurable)

**Create Database Table:**

Create a new table `notebook_images` in Supabase with the following schema:

```sql
create table notebook_images (
  id uuid primary key default gen_random_uuid(),
  note_id text not null,
  user_id text not null,
  file_path text not null,
  filename text not null,
  mime_type text not null,
  file_size bigint not null,
  alt_text text,
  caption text,
  is_deleted boolean default false,
  created_at timestamp with time zone default now(),
  updated_at timestamp with time zone default now()
);

create index idx_notebook_images_note_id on notebook_images(note_id);
create index idx_notebook_images_user_id on notebook_images(user_id);
create index idx_notebook_images_is_deleted on notebook_images(is_deleted);
```

**Row Level Security (RLS) Policies:**

```sql
-- Enable RLS
alter table notebook_images enable row level security;

-- Users can only view their own images
create policy "Users can view own images"
  on notebook_images for select
  using (auth.uid()::text = user_id);

-- Users can only insert their own images
create policy "Users can insert own images"
  on notebook_images for insert
  with check (auth.uid()::text = user_id);

-- Users can only update their own images
create policy "Users can update own images"
  on notebook_images for update
  using (auth.uid()::text = user_id);

-- Users can only delete their own images
create policy "Users can delete own images"
  on notebook_images for delete
  using (auth.uid()::text = user_id);
```

**Storage Bucket Policies:**

```sql
-- Users can upload to their own folder
create policy "Users can upload to own folder"
  on storage.objects for insert
  with check (
    bucket_id = 'notebook-images' AND
    (storage.foldername(name))[1] = auth.uid()::text
  );

-- Users can view their own images
create policy "Users can view own images"
  on storage.objects for select
  using (
    bucket_id = 'notebook-images' AND
    (storage.foldername(name))[1] = auth.uid()::text
  );

-- Users can delete their own images
create policy "Users can delete own images"
  on storage.objects for delete
  using (
    bucket_id = 'notebook-images' AND
    (storage.foldername(name))[1] = auth.uid()::text
  );
```

### 2. Frontend Implementation

**File: `lib/types/notebook.ts`**

Add image-related types:

```typescript
export interface NotebookImage {
  id: string;
  note_id: string;
  user_id: string;
  file_path: string;
  filename: string;
  mime_type: string;
  file_size: number;
  alt_text?: string | null;
  caption?: string | null;
  is_deleted: boolean;
  created_at: string;
  updated_at: string;
}

export interface NotebookImageUploadParams {
  file: File;
  note_id: string;
  alt_text?: string;
  caption?: string;
}
```

**File: `lib/services/notebook-images-service.ts` (new file)**

Create service that calls API routes (no direct Supabase calls):

```typescript
import { NotebookImage, NotebookImageUploadParams } from '@/lib/types/notebook';

class NotebookImagesService {
  async uploadImage(params: NotebookImageUploadParams): Promise<{ id: string; url: string }> {
    const formData = new FormData();
    formData.append('file', params.file);
    formData.append('note_id', params.note_id);
    if (params.alt_text) formData.append('alt_text', params.alt_text);
    if (params.caption) formData.append('caption', params.caption);

    const response = await fetch('/api/notebook/images/upload', {
      method: 'POST',
      body: formData,
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.error || 'Upload failed');
    }

    const data = await response.json();
    return { id: data.id, url: data.url };
  }

  async getImageUrl(imageId: string): Promise<string> {
    const response = await fetch(`/api/notebook/images/${imageId}`);

    if (!response.ok) {
      throw new Error('Failed to get image URL');
    }

    const data = await response.json();
    return data.url;
  }

  async getImagesByNoteId(noteId: string): Promise<NotebookImage[]> {
    const response = await fetch(`/api/notebook/images/note/${noteId}`);

    if (!response.ok) {
      throw new Error('Failed to get images for note');
    }

    const data = await response.json();
    return data.images || [];
  }

  async deleteImage(imageId: string): Promise<void> {
    const response = await fetch(`/api/notebook/images/${imageId}`, {
      method: 'DELETE',
    });

    if (!response.ok) {
      throw new Error('Failed to delete image');
    }
  }

  async deleteImagesByNoteId(noteId: string): Promise<void> {
    const response = await fetch(`/api/notebook/images/note/${noteId}`, {
      method: 'DELETE',
    });

    if (!response.ok) {
      console.error('Failed to delete images for note');
      // Don't throw - allow note deletion to continue
    }
  }
}

export const notebookImagesService = new NotebookImagesService();
```

**File: `components/notebook/BlockEditor.tsx`**

Update to include image upload handler:

```typescript
// Add import
import { notebookImagesService } from '@/lib/services/notebook-images-service';
import { toast } from 'sonner';

// In the component, add upload handler
const handleUpload = async (file: File) => {
  if (!docId) {
    toast.error('Cannot upload image: Note ID missing');
    throw new Error('Note ID is required');
  }

  try {
    const { url } = await notebookImagesService.uploadImage({
      file,
      note_id: docId
    });
    return url;
  } catch (error) {
    console.error('Image upload error:', error);
    toast.error('Failed to upload image');
    throw error;
  }
};

// Update editor creation
const editor: BlockNoteEditor = useCreateBlockNote({
  initialContent: initialContent
    ? (JSON.parse(initialContent) as PartialBlock[])
    : undefined,
  uploadFile: handleUpload, // Add this line
});
```

### 3. API Routes

**File: `app/api/notebook/images/upload/route.ts` (new file)**

Create Next.js API route for image uploads (alternative/backup to direct Supabase calls):

```typescript
import { NextRequest, NextResponse } from 'next/server';
import { createClient } from '@/lib/supabase/server';

export async function POST(request: NextRequest) {
  try {
    const supabase = await createClient();
    const { data: { user }, error: authError } = await supabase.auth.getUser();
    
    if (authError || !user) {
      return NextResponse.json({ error: 'Unauthorized' }, { status: 401 });
    }

    const formData = await request.formData();
    const file = formData.get('file') as File;
    const noteId = formData.get('note_id') as string;
    
    if (!file || !noteId) {
      return NextResponse.json({ error: 'Missing file or note_id' }, { status: 400 });
    }

    // Upload logic here (similar to service)
    
    return NextResponse.json({ success: true, id: imageId, url });
  } catch (error) {
    return NextResponse.json({ error: 'Upload failed' }, { status: 500 });
  }
}
```

**File: `app/api/notebook/images/[imageId]/route.ts` (new file)**

Get image URL by ID:

```typescript
import { NextRequest, NextResponse } from 'next/server';
import { createClient } from '@/lib/supabase/server';

export async function GET(
  request: NextRequest,
  { params }: { params: Promise<{ imageId: string }> }
) {
  try {
    const { imageId } = await params;
    const supabase = await createClient();
    
    const { data: { user }, error: authError } = await supabase.auth.getUser();
    if (authError || !user) {
      return NextResponse.json({ error: 'Unauthorized' }, { status: 401 });
    }

    // Get image and return signed URL
    
    return NextResponse.json({ url: signedUrl });
  } catch (error) {
    return NextResponse.json({ error: 'Failed to get image' }, { status: 500 });
  }
}
```

### 4. Cleanup on Note Deletion

**File: `lib/hooks/use-notebook.ts`**

Update the `useDeleteNote` hook to cleanup images:

```typescript
import { notebookImagesService } from '@/lib/services/notebook-images-service';

// In useDeleteNote hook
const deleteNote = async ({ id }: { id: string }) => {
  try {
    // Delete associated images first
    await notebookImagesService.deleteImagesByNoteId(id);
    
    // Then delete the note
    // ... existing delete logic
  } catch (error) {
    // Handle error
  }
};
```

### 5. Environment Variables

Ensure these are set in `.env.local`:

```bash
NEXT_PUBLIC_SUPABASE_URL=your-supabase-url
NEXT_PUBLIC_SUPABASE_PUBLISHABLE_OR_ANON_KEY=your-anon-key
```

### 6. Testing Checklist

- [ ] Create Supabase bucket and table
- [ ] Configure RLS policies
- [ ] Test image upload through BlockNote editor
- [ ] Verify images are stored in correct folder structure
- [ ] Test image retrieval and display
- [ ] Test note deletion removes images
- [ ] Verify only authenticated user can access their images
- [ ] Test error handling (network failures, auth errors)
- [ ] Check file size limits
- [ ] Verify cleanup on failed uploads

## Key Files to Create/Modify

**New Files:**

- `lib/services/notebook-images-service.ts`
- `app/api/notebook/images/upload/route.ts`
- `app/api/notebook/images/[imageId]/route.ts`

**Modified Files:**

- `lib/types/notebook.ts` (add image types)
- `components/notebook/BlockEditor.tsx` (add uploadFile handler)
- `lib/hooks/use-notebook.ts` (add image cleanup on delete)

## Notes

- Images are private by default via Supabase RLS
- Signed URLs expire after 1 hour (configurable)
- Folder structure: `{userId}/notebooks/{noteId}/{imageId}.ext`
- On note deletion, all associated images are deleted immediately
- BlockNote will handle image display automatically once URL is returned

### To-dos

- [ ] Create Supabase Storage bucket 'notebook-images' and configure RLS policies for private access
- [ ] Create 'notebook_images' table in Supabase with proper schema and RLS policies
- [ ] Add NotebookImage and NotebookImageUploadParams types to lib/types/notebook.ts
- [ ] Create lib/services/notebook-images-service.ts with upload, get, and delete methods
- [ ] Update components/notebook/BlockEditor.tsx to integrate image upload handler using uploadFile option
- [ ] Create API routes: app/api/notebook/images/upload/route.ts and app/api/notebook/images/[imageId]/route.ts
- [ ] Update lib/hooks/use-notebook.ts to cleanup images when note is deleted
- [ ] Test complete flow: upload image, display in editor, delete note, verify cleanup