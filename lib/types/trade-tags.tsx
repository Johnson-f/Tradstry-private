export interface TradeTag {
  id: string;
  category: string;
  name: string;
  color?: string | null;
  description?: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CreateTagRequest {
  category: string;
  name: string;
  color?: string;
  description?: string;
}

export interface UpdateTagRequest {
  category?: string;
  name?: string;
  color?: string;
  description?: string;
}

export interface TagQuery {
  category?: string;
  limit?: number;
  offset?: number;
}

export interface ApiListResponse<T> {
  success: boolean;
  message: string;
  data: T[] | null;
}

export interface ApiItemResponse<T> {
  success: boolean;
  message: string;
  data: T | null;
}

