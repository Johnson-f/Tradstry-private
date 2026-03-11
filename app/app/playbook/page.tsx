"use client";

import { useState, useEffect } from 'react';
import { Button } from '@/components/ui/button';
import { Plus, Search, MoreVertical, Edit, Trash2, Lock } from 'lucide-react';
import { Input } from '@/components/ui/input';
import { Badge } from '@/components/ui/badge';
import { Skeleton } from '@/components/ui/skeleton';
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu';

import { PlaybookCreateDialog } from '@/components/playbook/playbook-create-dialog';
import { PlaybookEditDialog } from '@/components/playbook/playbook-edit-dialog';
import { PlaybookDeleteDialog } from '@/components/playbook/playbook-delete-dialog';
import { usePlaybooks, useAllPlaybooksAnalytics } from '@/lib/hooks/use-playbooks';
import type { Playbook, PlaybookAnalytics } from '@/lib/types/playbook';
import { AppPageHeader } from '@/components/app-page-header';

function formatDateSafe(value?: string | null): string {
  const raw = value || '';
  const d = new Date(raw);
  return isNaN(d.getTime()) ? '' : d.toLocaleDateString();
}

function getCreatedAt(pb: Playbook): string | null {
  // Support both snake_case and legacy camelCase
  const snake = (pb as unknown as { created_at?: string }).created_at;
  if (typeof snake === 'string') return snake;
  const camel = (pb as unknown as { createdAt?: string }).createdAt;
  return typeof camel === 'string' ? camel : null;
}

function formatCurrencyUSD(value: number | null | undefined): string {
  if (value == null) return '$0.00';
  return new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' }).format(value);
}

function formatPercent(value: number | null | undefined): string {
  if (value == null) return '0.00%';
  return `${value.toFixed(2)}%`;
}

function formatNumber(value: number | null | undefined, decimals: number = 2): string {
  if (value == null) return '0.00';
  return value.toFixed(decimals);
}

function WinRateRing({ value }: { value: number | null | undefined }) {
  const safeValue = value ?? 0;
  const clamped = Math.max(0, Math.min(100, safeValue));
  const gradient = `conic-gradient(hsl(var(--primary)) ${clamped * 3.6}deg, hsl(var(--muted)) 0)`;
  return (
    <div className="flex items-center gap-3">
      <div className="relative h-10 w-10" aria-label="Win rate">
        <div className="h-10 w-10 rounded-full" style={{ background: gradient }} />
        <div className="absolute inset-1 rounded-full bg-background border" />
      </div>
      <div className="flex flex-col">
        <span className="text-xs text-muted-foreground">Win rate</span>
        <span className="font-semibold">{formatPercent(safeValue)}</span>
      </div>
    </div>
  );
}

export default function PlaybookPage() {
  const { data: playbooks = [], isLoading } = usePlaybooks();
  const { data: allAnalytics = [], isLoading: analyticsLoading } = useAllPlaybooksAnalytics();
  
  const [filteredPlaybooks, setFilteredPlaybooks] = useState<Playbook[]>([]);
  const [searchQuery, setSearchQuery] = useState('');
  const [isCreateDialogOpen, setIsCreateDialogOpen] = useState(false);
  const [isEditDialogOpen, setIsEditDialogOpen] = useState(false);
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);
  const [selectedPlaybook, setSelectedPlaybook] = useState<Playbook | null>(null);

  // Build analytics lookup by id for fast access
  const analyticsById = (allAnalytics || []).reduce<Record<string, PlaybookAnalytics>>((acc, a) => {
    acc[a.playbook_id] = a;
    return acc;
  }, {});

  // Filter playbooks when search or playbooks change
  useEffect(() => {
    let filtered = [...(playbooks || [])];

    // Filter by search query
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      filtered = filtered.filter(playbook =>
        playbook.name.toLowerCase().includes(query) ||
        playbook.description?.toLowerCase().includes(query)
      );
    }

    setFilteredPlaybooks(filtered);
  }, [playbooks, searchQuery]);

  const handlePlaybookCreated = () => {
    setIsCreateDialogOpen(false);
  };

  const handlePlaybookUpdated = () => {
    setIsEditDialogOpen(false);
  };

  const handlePlaybookDeleted = () => {
    setIsDeleteDialogOpen(false);
  };

  const handleEditPlaybook = (playbook: Playbook) => {
    setSelectedPlaybook(playbook);
    setIsEditDialogOpen(true);
  };

  const handleDeletePlaybook = (playbook: Playbook) => {
    setSelectedPlaybook(playbook);
    setIsDeleteDialogOpen(true);
  };

  if (isLoading) {
    return (
      <div className="h-screen flex flex-col">
        <div className="w-full border-b bg-background px-8 py-4 flex-shrink-0">
          <h1 className="text-2xl font-bold tracking-tight">Playbook</h1>
        </div>
        <div className="flex-1 overflow-hidden">
          <div className="h-full overflow-y-auto">
            <div className="p-8">
              <div className="space-y-4">
                {[...Array(5)].map((_, i) => (
                  <div key={i} className="flex items-center space-x-4">
                    <Skeleton className="h-12 w-12 rounded-full" />
                    <div className="space-y-2">
                      <Skeleton className="h-4 w-[250px]" />
                      <Skeleton className="h-4 w-[200px]" />
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      </div>
    );
  }

  const headerActions = (
    <Button onClick={() => setIsCreateDialogOpen(true)}>
      <Plus className="mr-2 h-4 w-4" />
      New Playbook
    </Button>
  );

  return (
    <div className="h-screen flex flex-col">
      <AppPageHeader title="Playbook" actions={headerActions} />

      {/* Search */}
      <div className="border-b bg-background px-8 py-4 flex-shrink-0">
        <div className="flex items-center space-x-4">
          <div className="relative flex-1 max-w-md">
            <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <Input
              placeholder="Search playbooks..."
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="pl-10"
            />
          </div>
        </div>
      </div>

      {/* Main Content */}
      <div className="flex-1 overflow-hidden">
        <div className="h-full overflow-y-auto">
          <div className="p-8">
            {filteredPlaybooks.length === 0 ? (
              <div className="text-center py-12">
                <div className="text-muted-foreground mb-4">
                  {(playbooks || []).length === 0 ? (
                    <>
                      <h3 className="text-lg font-semibold mb-2">No playbooks created yet</h3>
                      <p className="text-sm">Create your first trading playbook to get started</p>
                    </>
                  ) : (
                    <>
                      <h3 className="text-lg font-semibold mb-2">No playbooks match your search</h3>
                      <p className="text-sm">Try adjusting your search criteria</p>
                    </>
                  )}
                </div>
                {(playbooks || []).length === 0 && (
                  <Button onClick={() => setIsCreateDialogOpen(true)}>
                    <Plus className="mr-2 h-4 w-4" />
                    Create First Playbook
                  </Button>
                )}
              </div>
            ) : (
              <div className="grid gap-6 md:grid-cols-2 lg:grid-cols-3">
                {filteredPlaybooks.map((playbook) => {
                  const createdIso = getCreatedAt(playbook);
                  const created = formatDateSafe(createdIso);
                  const a = analyticsById[playbook.id];

                  return (
                    <div
                      key={playbook.id}
                      className="border rounded-lg p-6 hover:shadow-md transition-shadow"
                    >
                      <div className="flex items-start justify-between mb-4">
                        <div className="flex-1">
                          <div className="flex items-center gap-3 mb-2">
                            <div
                              className="h-10 w-10 rounded-full border flex items-center justify-center text-xl"
                              style={{ backgroundColor: playbook.color || 'var(--muted)' }}
                              aria-label="Playbook icon"
                            >
                              {playbook.emoji || ''}
                            </div>
                            <h3 className="font-semibold text-lg">{playbook.name}</h3>
                          </div>
                          <div className="flex items-center gap-2 text-xs text-muted-foreground">
                            <Badge variant="secondary">Trading Setup</Badge>
                            <span>•</span>
                            <div className="inline-flex items-center gap-1">
                              <Lock className="h-3.5 w-3.5" />
                              <span>Private</span>
                            </div>
                          </div>
                        </div>
                        <div className="flex items-center space-x-2">
                          <DropdownMenu>
                            <DropdownMenuTrigger asChild>
                              <Button variant="ghost" size="sm" className="h-8 w-8 p-0">
                                <MoreVertical className="h-4 w-4" />
                              </Button>
                            </DropdownMenuTrigger>
                            <DropdownMenuContent align="end">
                              <DropdownMenuItem onClick={() => handleEditPlaybook(playbook)}>
                                <Edit className="mr-2 h-4 w-4" />
                                Edit
                              </DropdownMenuItem>
                              <DropdownMenuItem 
                                onClick={() => handleDeletePlaybook(playbook)}
                                className="text-destructive"
                              >
                                <Trash2 className="mr-2 h-4 w-4" />
                                Delete
                              </DropdownMenuItem>
                            </DropdownMenuContent>
                          </DropdownMenu>
                        </div>
                      </div>
                      
                      {a ? (
                        <div className="mb-4">
                          <button className="text-primary text-sm font-medium hover:underline">
                            {a.total_trades} trades
                          </button>
                        </div>
                      ) : null}

                      {a ? (
                        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
                          <div className="flex items-center">
                            <WinRateRing value={a.win_rate} />
                          </div>
                          <div className="flex flex-col">
                            <span className="text-xs text-muted-foreground">Net P&L</span>
                            <span className="font-semibold">{formatCurrencyUSD(a.net_pnl)}</span>
                          </div>
                          <div className="flex flex-col">
                            <span className="text-xs text-muted-foreground">Profit factor</span>
                            <span className="font-semibold">{formatNumber(a.profit_factor)}</span>
                          </div>
                          <div className="flex flex-col">
                            <span className="text-xs text-muted-foreground">Missed trades</span>
                            <span className="font-semibold">{a.missed_trades ?? 0}</span>
                          </div>
                          <div className="flex flex-col">
                            <span className="text-xs text-muted-foreground">Expectancy</span>
                            <span className="font-semibold">{formatCurrencyUSD(a.expectancy)}</span>
                          </div>
                          <div className="flex flex-col">
                            <span className="text-xs text-muted-foreground">Average winner</span>
                            <span className="font-semibold">{formatCurrencyUSD(a.average_winner)}</span>
                          </div>
                          <div className="flex flex-col">
                            <span className="text-xs text-muted-foreground">Average loser</span>
                            <span className="font-semibold">{formatCurrencyUSD(a.average_loser)}</span>
                          </div>
                        </div>
                      ) : (
                        <div className="text-sm text-muted-foreground mb-4">
                          {analyticsLoading ? 'Loading analytics…' : 'No analytics available'}
                        </div>
                      )}
                      
                      <div className="mt-4 text-xs text-muted-foreground">
                        {created ? `Created ${created}` : 'Created date unavailable'}
                      </div>
                    </div>
                  );
                })}
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Create Playbook Dialog */}
      <PlaybookCreateDialog
        open={isCreateDialogOpen}
        onOpenChange={setIsCreateDialogOpen}
        onPlaybookCreated={handlePlaybookCreated}
      />

      {/* Edit Playbook Dialog */}
      <PlaybookEditDialog
        open={isEditDialogOpen}
        onOpenChange={setIsEditDialogOpen}
        playbook={selectedPlaybook}
        onPlaybookUpdated={handlePlaybookUpdated}
      />

      {/* Delete Playbook Dialog */}
      <PlaybookDeleteDialog
        open={isDeleteDialogOpen}
        onOpenChange={setIsDeleteDialogOpen}
        playbook={selectedPlaybook}
        onPlaybookDeleted={handlePlaybookDeleted}
      />
    </div>
  );
}