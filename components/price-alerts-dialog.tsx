"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Form,
  FormControl,
  FormField,
  FormItem,
  FormLabel,
  FormMessage,
} from "@/components/ui/form";
import { Input } from "@/components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { useForm } from "react-hook-form";
import { z } from "zod";
import { zodResolver } from "@hookform/resolvers/zod";
import { alertsService } from "@/lib/services/alerts-service";
import type { AlertRule } from "@/lib/types/alerts";
import { toast } from "sonner";
import { Loader2, Plus, RefreshCw, Trash2 } from "lucide-react";
import { useSimpleQuotes } from "@/lib/hooks/use-market-data-service";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";

const schema = z.object({
  symbol: z.string().min(1, "Symbol is required").transform((v) => v.toUpperCase()),
  operator: z.enum([">=", "<="]),
  threshold: z.coerce.number().positive("Threshold must be positive"),
  note: z
    .string()
    .max(200, "Keep notes under 200 characters")
    .transform((v) => v.trim())
    .optional()
    .or(z.literal("")),
  maxTriggers: z
    .preprocess(
      (v) => (v === "" || v === null || v === undefined ? undefined : Number(v)),
      z.number().positive("Must be greater than 0").int("Must be a whole number").optional()
    ),
});

type FormValues = z.infer<typeof schema>;

interface PriceAlertsDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function PriceAlertsDialog({ open, onOpenChange }: PriceAlertsDialogProps) {
  const [rules, setRules] = useState<AlertRule[]>([]);
  const [loading, setLoading] = useState(false);
  const [creating, setCreating] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);

  const symbols = useMemo(
    () =>
      rules
        .filter((r) => r.alert_type === "price" && r.symbol)
        .map((r) => r.symbol as string),
    [rules]
  );

  const { quotes, isLoading: quotesLoading } = useSimpleQuotes(
    { symbols },
    open && symbols.length > 0
  );

  const quoteMap = useMemo(() => {
    const map: Record<string, number> = {};
    const list = Array.isArray(quotes) ? quotes : [];

    const parseNumeric = (value: unknown): number | undefined => {
      if (typeof value === "number") return value;
      if (typeof value === "string") {
        // Remove currency symbols/commas and parse
        const cleaned = value.replace(/[^0-9.\-]/g, "");
        const parsed = Number.parseFloat(cleaned);
        return Number.isNaN(parsed) ? undefined : parsed;
      }
      return undefined;
    };

    list.forEach((q: any) => {
      if (!q?.symbol) return;
      const symbolKey = q.symbol.toUpperCase();
      const candidate =
        parseNumeric(q?.last) ??
        parseNumeric(q?.price) ??
        parseNumeric(q?.regularMarketPrice) ??
        parseNumeric(q?.regular_market_price);

      if (candidate !== undefined) {
        map[symbolKey] = candidate;
      }
    });

    return map;
  }, [quotes]);

  const form = useForm<FormValues>({
    // @ts-ignore
    resolver: zodResolver(schema),
    defaultValues: {
      symbol: "",
      operator: ">=",
      threshold: 0,
      note: "",
      maxTriggers: 5,
    },
  });

  const loadRules = useCallback(async () => {
    setLoading(true);
    try {
      const res = await alertsService.listRules();
      setRules(res.filter((r) => r.alert_type === "price"));
    } catch (error) {
      console.error("Failed to load price alerts", error);
      toast.error("Failed to load price alerts");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (open) {
      void loadRules();
    }
  }, [open, loadRules]);

  const onSubmit = useCallback(
    async (values: FormValues) => {
      setCreating(true);
      try {
        const payload = {
          alert_type: "price" as const,
          symbol: values.symbol,
          operator: values.operator,
          threshold: values.threshold,
          channels: ["push"] as const,
          cooldown_seconds: 0,
          note: values.note?.trim() ? values.note.trim() : undefined,
          max_triggers: values.maxTriggers ?? undefined,
        };
        const created = await alertsService.createRule(payload as any);
        setRules((prev) => [created, ...prev]);
        form.reset({ symbol: "", operator: values.operator, threshold: 0, note: "", maxTriggers: values.maxTriggers });
        toast.success("Price alert created");
      } catch (error) {
        console.error("Failed to create price alert", error);
        toast.error("Failed to create price alert");
      } finally {
        setCreating(false);
      }
    },
    [form]
  );

  const handleDelete = useCallback(
    async (id: string) => {
      setDeletingId(id);
      try {
        await alertsService.deleteRule(id);
        setRules((prev) => prev.filter((r) => r.id !== id));
        toast.success("Alert deleted");
      } catch (error) {
        console.error("Failed to delete alert", error);
        toast.error("Failed to delete alert");
      } finally {
        setDeletingId(null);
      }
    },
    []
  );

  const renderRule = (rule: AlertRule) => {
    const symbol = (rule.symbol || "").toUpperCase();
    const price = quoteMap[symbol];
    const max = rule.max_triggers ?? null;
    const fired = rule.fired_count ?? 0;
    const remaining = max ? Math.max(0, max - fired) : null;
    const isExpired = !rule.is_active || (max !== null && fired >= max);
    return (
      <div key={rule.id} className="flex items-center justify-between gap-3 rounded-lg border px-3 py-2">
        <div className="space-y-1">
          <div className="flex items-center gap-2">
            <Badge variant="secondary">{symbol || "—"}</Badge>
            <Badge>{`${rule.operator} ${rule.threshold ?? ""}`}</Badge>
            {isExpired && <Badge variant="destructive">Expired</Badge>}
          </div>
          <p className="text-xs text-muted-foreground">
            Channel: {rule.channels?.join(", ") || "push"} • Created {new Date(rule.created_at).toLocaleString()}
          </p>
          {(rule.note || max !== null) && (
            <p className="text-xs text-muted-foreground">
              {rule.note ? `Note: ${rule.note}` : null}
              {rule.note && max !== null ? " • " : null}
              {max !== null ? `Remaining: ${remaining} / ${max}` : null}
            </p>
          )}
          {rule.last_fired_at && (
            <p className="text-xs text-muted-foreground">
              Last fired {new Date(rule.last_fired_at).toLocaleString()} • Sent {fired}
            </p>
          )}
        </div>
        <div className="flex items-center gap-3">
        <div className="text-right">
          <p className="text-sm font-semibold">
            {price !== undefined ? `$${price.toFixed(2)}` : quotesLoading ? "Loading..." : "—"}
          </p>
          </div>
          <Button
            variant="ghost"
            size="icon"
            onClick={() => handleDelete(rule.id)}
            disabled={deletingId === rule.id}
          >
            {deletingId === rule.id ? <Loader2 className="h-4 w-4 animate-spin" /> : <Trash2 className="h-4 w-4" />}
          </Button>
        </div>
      </div>
    );
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            Price alerts
          </DialogTitle>
        </DialogHeader>

        <div className="grid gap-4">
          <div className="flex items-center justify-between">
            <p className="text-sm text-muted-foreground">
              Create new price alerts and see current quotes for your tracked symbols.
            </p>
            <Button variant="ghost" size="sm" onClick={loadRules} disabled={loading}>
              {loading ? <Loader2 className="mr-2 h-4 w-4 animate-spin" /> : <RefreshCw className="mr-2 h-4 w-4" />}
              Refresh
            </Button>
          </div>

          <Form {...form}>
            {/* @ts-ignore */}
            <form onSubmit={form.handleSubmit(onSubmit)} className="grid grid-cols-1 gap-3 sm:grid-cols-5 sm:items-end">
              <FormField
              // @ts-ignore
                control={form.control}
                name="symbol"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Symbol</FormLabel>
                    <FormControl>
                      <Input placeholder="AAPL" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                // @ts-ignore 
                control={form.control}
                name="operator"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Operator</FormLabel>
                    <Select onValueChange={field.onChange} defaultValue={field.value}>
                      <FormControl>
                        <SelectTrigger>
                          <SelectValue placeholder="Choose" />
                        </SelectTrigger>
                      </FormControl>
                      <SelectContent>
                        <SelectItem value=">=">&ge;=</SelectItem>
                        <SelectItem value="<=">&le;=</SelectItem>
                      </SelectContent>
                    </Select>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                // @ts-ignore
                control={form.control}
                name="threshold"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Price</FormLabel>
                    <FormControl>
                      <Input type="number" step="0.01" placeholder="300" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                // @ts-ignore
                control={form.control}
                name="maxTriggers"
                render={({ field }) => (
                  <FormItem>
                    <FormLabel>Max alerts</FormLabel>
                    <FormControl>
                      <Input type="number" min={1} step={1} placeholder="5" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <FormField
                // @ts-ignore
                control={form.control}
                name="note"
                render={({ field }) => (
                  <FormItem className="sm:col-span-2">
                    <FormLabel>Note (optional)</FormLabel>
                    <FormControl>
                      <Input placeholder="Reminder for this alert" {...field} />
                    </FormControl>
                    <FormMessage />
                  </FormItem>
                )}
              />
              <div className="sm:self-end">
                <Button type="submit" disabled={creating} className="w-full sm:w-auto">
                  {creating ? (
                    <>
                      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                      Saving...
                    </>
                  ) : (
                    <>
                      <Plus className="mr-2 h-4 w-4" />
                      Add alert
                    </>
                  )}
                </Button>
              </div>
            </form>
          </Form>

          <Separator />

          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <p className="text-sm font-medium">Your price alerts</p>
              {quotesLoading && <Loader2 className="h-4 w-4 animate-spin text-muted-foreground" />}
            </div>
            <ScrollArea className="max-h-80 pr-2">
              <div className={cn("space-y-2", { "opacity-60": loading })}>
                {rules.length === 0 && !loading ? (
                  <p className="text-sm text-muted-foreground">No price alerts yet.</p>
                ) : (
                  rules.map(renderRule)
                )}
              </div>
            </ScrollArea>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

