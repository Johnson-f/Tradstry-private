'use client';

import React, { useState, useMemo } from 'react';
import { Badge } from '@/components/ui/badge';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Label } from '@/components/ui/label';
import { ScrollArea } from '@/components/ui/scroll-area';
import { X, GripVertical, Plus, ChevronDown, ChevronUp } from 'lucide-react';
import { useTradeTags, useTradeTagsForTrade, useTradeTagCategories } from '@/lib/hooks/use-trade-tag';
import { useTradeTagCrud } from '@/lib/hooks/use-trade-tag';
import { toast } from 'sonner';
import type { TradeTag } from '@/lib/types/trade-tags';

interface TradeTagsManagerProps {
  tradeId: number;
  tradeType: 'stock' | 'option';
}

// Built-in categories that should always show
const BUILT_IN_CATEGORIES = ['Mistakes', 'Setups', 'Custom Tags', 'Habits'];

// Generate color for category (consistent based on name)
const getCategoryColor = (category: string): string => {
  const colors = [
    'bg-yellow-500',
    'bg-purple-500',
    'bg-green-500',
    'bg-blue-500',
    'bg-red-500',
    'bg-indigo-500',
    'bg-pink-500',
    'bg-teal-500',
  ];
  let hash = 0;
  for (let i = 0; i < category.length; i++) {
    hash = category.charCodeAt(i) + ((hash << 5) - hash);
  }
  return colors[Math.abs(hash) % colors.length];
};

export default function TradeTagsManager({ tradeId, tradeType }: TradeTagsManagerProps) {
  const [selectedCategory, setSelectedCategory] = useState<Record<string, string>>({});
  const [expandedCategories, setExpandedCategories] = useState<Record<string, boolean>>({});
  
  // Dialogs
  const [showAddCategoryDialog, setShowAddCategoryDialog] = useState(false);
  const [showManageTagsDialog, setShowManageTagsDialog] = useState(false);
  const [newCategoryName, setNewCategoryName] = useState('');
  const [manageTagsState, setManageTagsState] = useState<Record<string, { newTagName: string }>>({});

  // Fetch data
  const { data: allTags = [], isLoading: isLoadingTags } = useTradeTags();
  const { data: categories = [] } = useTradeTagCategories();
  const { tags: tradeTags = [], isLoading: isLoadingTradeTags, addTags, removeTag } = useTradeTagsForTrade({ tradeType, tradeId });
  const { createTag, deleteTag } = useTradeTagCrud();

  // Get tags by category
  const tagsByCategory = useMemo(() => {
    const grouped: Record<string, TradeTag[]> = {};
    allTags.forEach(tag => {
      if (!grouped[tag.category]) {
        grouped[tag.category] = [];
      }
      grouped[tag.category].push(tag);
    });
    return grouped;
  }, [allTags]);

  // Get selected tags for this trade by category
  const tradeTagsByCategory = useMemo(() => {
    const grouped: Record<string, TradeTag[]> = {};
    tradeTags.forEach(tag => {
      if (!grouped[tag.category]) {
        grouped[tag.category] = [];
      }
      grouped[tag.category].push(tag);
    });
    return grouped;
  }, [tradeTags]);

  // Handle adding a tag to trade
  const handleAddTag = async (tagId: string) => {
    try {
      await addTags([tagId]);
      setSelectedCategory({});
      toast.success('Tag added');
    } catch {
      toast.error('Failed to add tag');
    }
  };

  // Handle removing a tag from trade
  const handleRemoveTag = async (tagId: string) => {
    try {
      await removeTag(tagId);
      toast.success('Tag removed');
    } catch {
      toast.error('Failed to remove tag');
    }
  };

  // Handle creating a new category (creates first tag in that category)
  const handleCreateCategory = async () => {
    if (!newCategoryName.trim()) {
      toast.error('Category name is required');
      return;
    }

    try {
      // Create a tag with the new category (tags define categories)
      await createTag.mutateAsync({
        category: newCategoryName.trim(),
        name: `Default tag for ${newCategoryName.trim()}`,
      });
      setNewCategoryName('');
      setShowAddCategoryDialog(false);
      toast.success('Category created');
    } catch {
      toast.error('Failed to create category');
    }
  };

  // Handle creating tag in manage dialog
  const handleCreateTagInCategory = async (category: string) => {
    const tagName = manageTagsState[category]?.newTagName?.trim();
    if (!tagName) {
      toast.error('Tag name is required');
      return;
    }

    try {
      await createTag.mutateAsync({
        category,
        name: tagName,
      });
      setManageTagsState({ ...manageTagsState, [category]: { newTagName: '' } });
      toast.success('Tag created');
    } catch {
      toast.error('Failed to create tag');
    }
  };

  // Display categories (only from API, include built-ins if they exist or show them anyway)
  const displayCategories = useMemo(() => {
    const categorySet = new Set(categories);
    // Add built-in categories if they don't exist yet
    BUILT_IN_CATEGORIES.forEach(cat => {
      if (!categorySet.has(cat)) {
        categorySet.add(cat);
      }
    });
    return Array.from(categorySet).sort();
  }, [categories]);

  // Toggle category expansion
  const toggleCategory = (category: string) => {
    setExpandedCategories({
      ...expandedCategories,
      [category]: !expandedCategories[category],
    });
  };

  if (isLoadingTags || isLoadingTradeTags) {
    return (
      <div className="space-y-4">
        {[1, 2, 3, 4].map(i => (
          <div key={i} className="h-20 bg-gray-100 animate-pulse rounded" />
        ))}
      </div>
    );
  }

  return (
    <>
      <div className="space-y-4">
        {displayCategories.map(category => {
          const categoryColor = getCategoryColor(category);
          const categoryTags = tagsByCategory[category] || [];
          const tradeTagsInCategory = tradeTagsByCategory[category] || [];
          const availableTags = categoryTags.filter(
            tag => !tradeTagsInCategory.some(t => t.id === tag.id)
          );
          const isExpanded = expandedCategories[category];

          return (
            <div key={category} className="space-y-3">
              {/* Category Header - Clickable to expand/collapse */}
              <button
                onClick={() => toggleCategory(category)}
                className="w-full flex items-center justify-between hover:bg-gray-50 rounded p-2"
              >
                <div className="flex items-center gap-2">
                  <GripVertical className="h-4 w-4 text-muted-foreground" />
                  <div className={`w-3 h-3 rounded-full ${categoryColor}`} />
                  <span className="text-sm font-medium">{category}</span>
                  <Badge variant="outline" className="text-xs">
                    {tradeTagsInCategory.length} selected
                  </Badge>
                </div>
                {isExpanded ? (
                  <ChevronUp className="h-4 w-4 text-muted-foreground" />
                ) : (
                  <ChevronDown className="h-4 w-4 text-muted-foreground" />
                )}
              </button>

              {/* Selected Tags - Always visible */}
              {tradeTagsInCategory.length > 0 && (
                <div className="flex flex-wrap gap-2">
                  {tradeTagsInCategory.map(tag => (
                    <Badge
                      key={tag.id}
                      variant="secondary"
                      className="flex items-center gap-1"
                    >
                      {tag.name}
                      <button
                        onClick={() => handleRemoveTag(tag.id)}
                        className="ml-1 hover:text-destructive"
                      >
                        <X className="h-3 w-3" />
                      </button>
                    </Badge>
                  ))}
                </div>
              )}

              {/* Select Component - Shows when expanded */}
              {isExpanded && (
                <div className="space-y-2">
                  {availableTags.length > 0 ? (
                    <Select
                      value={selectedCategory[category] || ''}
                      onValueChange={(value) => {
                        if (value) {
                          handleAddTag(value);
                          setSelectedCategory({ ...selectedCategory, [category]: '' });
                        }
                      }}
                    >
                      <SelectTrigger>
                        <SelectValue placeholder="Select a tag from this category" />
                      </SelectTrigger>
                      <SelectContent>
                        {availableTags.map(tag => (
                          <SelectItem key={tag.id} value={tag.id}>
                            {tag.name}
                          </SelectItem>
                        ))}
                      </SelectContent>
                    </Select>
                  ) : (
                    <p className="text-sm text-muted-foreground">
                      No available tags in this category. Create one in Manage Tags.
                    </p>
                  )}
                </div>
              )}
            </div>
          );
        })}

        {/* Bottom Actions */}
          <div className="flex items-center justify-between pt-4 border-t">
            <Button
              variant="ghost"
              onClick={() => setShowAddCategoryDialog(true)}
              className="text-sm"
            >
              Add new category
            </Button>
            <Button
              variant="ghost"
              onClick={() => setShowManageTagsDialog(true)}
              className="text-sm"
            >
              Manage tags
            </Button>
          </div>
      </div>

      {/* Add New Category Dialog */}
      <Dialog open={showAddCategoryDialog} onOpenChange={setShowAddCategoryDialog}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Add New Category</DialogTitle>
            <DialogDescription>
              Create a new category for organizing your trade tags.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="category-name">Category Name</Label>
              <Input
                id="category-name"
                value={newCategoryName}
                onChange={(e) => setNewCategoryName(e.target.value)}
                placeholder="e.g., Emotions, Strategies"
                onKeyDown={(e) => {
                  if (e.key === 'Enter') {
                    handleCreateCategory();
                  }
                }}
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setShowAddCategoryDialog(false)}>
              Cancel
            </Button>
            <Button onClick={handleCreateCategory} disabled={createTag.isPending}>
              {createTag.isPending ? 'Creating...' : 'Create Category'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Manage Tags Dialog */}
      <Dialog open={showManageTagsDialog} onOpenChange={setShowManageTagsDialog}>
        <DialogContent className="max-w-2xl max-h-[600px] flex flex-col">
          <DialogHeader>
            <DialogTitle>Manage Tags</DialogTitle>
            <DialogDescription>
              View, create, and delete your trade tags across all categories.
            </DialogDescription>
          </DialogHeader>
          <ScrollArea className="flex-1 pr-4">
            <div className="space-y-6 py-4">
              {displayCategories.map(category => {
                const categoryTags = tagsByCategory[category] || [];
                const categoryColor = getCategoryColor(category);
                const newTagName = manageTagsState[category]?.newTagName || '';

                return (
                  <div key={category} className="space-y-3 border-b pb-4 last:border-b-0">
                    <div className="flex items-center gap-2">
                      <div className={`w-3 h-3 rounded-full ${categoryColor}`} />
                      <h4 className="font-medium text-sm">{category}</h4>
                    </div>

                    {/* Existing Tags */}
                    {categoryTags.length > 0 && (
                      <div className="flex flex-wrap gap-2">
                        {categoryTags.map(tag => (
                          <Badge
                            key={tag.id}
                            variant="secondary"
                            className="flex items-center gap-2"
                          >
                            {tag.name}
                            <button
                              onClick={async () => {
                                if (confirm(`Delete tag "${tag.name}"?`)) {
                                  try {
                                    await deleteTag.mutateAsync(tag.id);
                                    toast.success('Tag deleted');
                                  } catch {
                                    toast.error('Failed to delete tag');
                                  }
                                }
                              }}
                              className="ml-1 hover:text-destructive"
                            >
                              <X className="h-3 w-3" />
                            </button>
                          </Badge>
                        ))}
                      </div>
                    )}

                    {/* Create New Tag in Category */}
                    <div className="flex items-center gap-2">
                      <Input
                        placeholder={`New tag in ${category}...`}
                        value={newTagName}
                        onChange={(e) => {
                          setManageTagsState({
                            ...manageTagsState,
                            [category]: { newTagName: e.target.value },
                          });
                        }}
                        onKeyDown={(e) => {
                          if (e.key === 'Enter' && newTagName.trim()) {
                            handleCreateTagInCategory(category);
                          }
                        }}
                        className="flex-1"
                      />
                      <Button
                        onClick={() => handleCreateTagInCategory(category)}
                        disabled={!newTagName.trim() || createTag.isPending}
                        size="icon"
                      >
                        <Plus className="h-4 w-4" />
                      </Button>
                    </div>
                  </div>
                );
              })}
            </div>
          </ScrollArea>
          <DialogFooter>
            <Button onClick={() => setShowManageTagsDialog(false)}>Close</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  );
}

