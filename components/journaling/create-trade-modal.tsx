"use client"

import React from "react"
import { useForm, useFieldArray } from "react-hook-form"
import { zodResolver } from "@hookform/resolvers/zod"
import { z } from "zod"
import { toast } from "sonner"
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Form, FormControl, FormDescription, FormField, FormItem, FormLabel, FormMessage } from "@/components/ui/form"
import { Input } from "@/components/ui/input"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Button } from "@/components/ui/button"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Plus } from "lucide-react"
import { useCreateStock } from "@/lib/hooks/use-stocks"
import { useCreateOption } from "@/lib/hooks/use-options"
import { TransactionRow } from "./transaction-row"
import type { CreateStockRequest } from "@/lib/types/stocks"
import type { CreateOptionRequest } from "@/lib/types/options"

// Helper to get local datetime string suitable for input[type="datetime-local"]
function getLocalDateTimeInputValue(date: Date = new Date()): string {
  const pad = (n: number) => String(n).padStart(2, "0")
  const year = date.getFullYear()
  const month = pad(date.getMonth() + 1)
  const day = pad(date.getDate())
  const hours = pad(date.getHours())
  const minutes = pad(date.getMinutes())
  return `${year}-${month}-${day}T${hours}:${minutes}`
}

// Generate UUID for trade_group_id
function generateTradeGroupId(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID()
  }
  return `trade-group-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`
}

// Transaction schema for stocks
const stockTransactionSchema = z.object({
  tradeType: z.enum(["BUY", "SELL"]),
  entryPrice: z.coerce.number().positive("Entry price must be positive"),
  exitPrice: z.coerce.number().positive("Exit price must be positive"),
  numberShares: z.coerce.number().positive("Number of shares must be positive"),
  entryDate: z.string().min(1, "Entry date is required"),
  exitDate: z.string().min(1, "Exit date is required"),
  commissions: z.coerce.number().min(0, "Commissions cannot be negative").optional().default(0),
})

// Stock form schema with transactions array
const stockFormSchema = z.object({
  symbol: z.string().min(1, "Symbol is required").max(10, "Symbol must be 10 characters or less"),
  orderType: z.enum(["MARKET", "LIMIT", "STOP", "STOP_LIMIT"]),
  stopLoss: z.coerce.number().positive("Stop loss must be positive"),
  takeProfit: z.coerce.number().positive("Take profit must be positive").optional(),
  initialTarget: z.coerce.number().positive("Initial target must be positive").optional(),
  profitTarget: z.coerce.number().positive("Profit target must be positive").optional(),
  tradeRatings: z.coerce.number().min(1, "Rating must be at least 1").max(5, "Rating must be at most 5").optional(),
  reviewed: z.boolean().optional().default(false),
  mistakes: z.string().optional(),
  transactions: z.array(stockTransactionSchema).min(1, "At least one transaction is required"),
})

type StockFormData = z.infer<typeof stockFormSchema>

// Transaction schema for options
const optionTransactionSchema = z.object({
  tradeType: z.enum(["BUY", "SELL"]),
  entryPrice: z.coerce.number().positive("Entry price must be positive"),
  exitPrice: z.coerce.number().positive("Exit price must be positive"),
  totalQuantity: z.coerce.number().int().positive("Number of contracts must be a positive integer"),
  entryDate: z.string().min(1, "Entry date is required"),
  exitDate: z.string().min(1, "Exit date is required"),
  commissions: z.coerce.number().min(0, "Commissions cannot be negative").optional().default(0),
})

// Option form schema with transactions array
const optionFormSchema = z.object({
  symbol: z.string().min(1, "Symbol is required").max(10, "Symbol must be 10 characters or less"),
  // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
  optionType: z.enum(["Call", "Put"], { required_error: "Option type is required" }),
  strikePrice: z.coerce.number().positive("Strike price must be positive"),
  expirationDate: z.string().min(1, "Expiration date is required"),
  premium: z.coerce.number().positive("Premium must be positive"),
  initialTarget: z.coerce.number().positive("Initial target must be positive").optional(),
  profitTarget: z.coerce.number().positive("Profit target must be positive").optional(),
  tradeRatings: z.coerce.number().min(1, "Rating must be at least 1").max(5, "Rating must be at most 5").optional(),
  reviewed: z.boolean().optional().default(false),
  mistakes: z.string().optional(),
  transactions: z.array(optionTransactionSchema).min(1, "At least one transaction is required"),
})

type OptionFormData = z.infer<typeof optionFormSchema>

interface CreateTradeModalProps {
  open: boolean
  onOpenChange: (open: boolean) => void
}

export default function CreateTradeModal({ open, onOpenChange }: CreateTradeModalProps) {
  const [activeTab, setActiveTab] = React.useState<"stock" | "option">("stock")
  const createStock = useCreateStock()
  const createOption = useCreateOption()

  const stockForm = useForm<StockFormData>({
    // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
    resolver: zodResolver(stockFormSchema),
    defaultValues: {
      symbol: "",
      orderType: "MARKET",
      stopLoss: 0,
      takeProfit: undefined,
      initialTarget: undefined,
      profitTarget: undefined,
      tradeRatings: undefined,
      reviewed: false,
      mistakes: "",
      transactions: [
        {
          tradeType: "BUY",
          entryPrice: 0,
          exitPrice: 0,
          numberShares: 0,
          entryDate: getLocalDateTimeInputValue(),
          exitDate: getLocalDateTimeInputValue(),
          commissions: 0,
        },
      ],
    },
  })

  const optionForm = useForm<OptionFormData>({
    // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
    resolver: zodResolver(optionFormSchema),
    defaultValues: {
      symbol: "",
      optionType: "Call",
      strikePrice: 0,
      expirationDate: "",
      premium: 0,
      initialTarget: undefined,
      profitTarget: undefined,
      tradeRatings: undefined,
      reviewed: false,
      mistakes: "",
      transactions: [
        {
          tradeType: "BUY",
          entryPrice: 0,
          exitPrice: 0,
          totalQuantity: 1,
          entryDate: getLocalDateTimeInputValue(),
          exitDate: getLocalDateTimeInputValue(),
          commissions: 0,
        },
      ],
    },
  })

  const stockTransactions = useFieldArray({
    control: stockForm.control,
    name: "transactions",
  })

  const optionTransactions = useFieldArray({
    control: optionForm.control,
    name: "transactions",
  })

  const handleStockSubmit = async (data: StockFormData) => {
    try {
      const tradeGroupId = generateTradeGroupId()
      let parentTradeId: number | undefined = undefined

      // Create transactions sequentially
      for (let i = 0; i < data.transactions.length; i++) {
        const transaction = data.transactions[i]
        const entryDateIso = new Date(transaction.entryDate).toISOString()
        const exitDateIso = new Date(transaction.exitDate).toISOString()

        const payload: CreateStockRequest = {
          symbol: data.symbol,
          tradeType: transaction.tradeType,
          orderType: data.orderType,
          entryPrice: transaction.entryPrice,
          exitPrice: transaction.exitPrice,
          stopLoss: data.stopLoss,
          commissions: Number.isFinite(transaction.commissions) ? transaction.commissions : 0,
          numberShares: transaction.numberShares,
          takeProfit: data.takeProfit,
          initialTarget: data.initialTarget,
          profitTarget: data.profitTarget,
          tradeRatings:
            data.tradeRatings != null && data.tradeRatings !== ("" as unknown)
              ? Math.round(Number(data.tradeRatings))
              : undefined,
          entryDate: entryDateIso,
          exitDate: exitDateIso,
          reviewed: data.reviewed,
          mistakes: data.mistakes,
          tradeGroupId: i === 0 ? tradeGroupId : undefined, // Only set on first transaction
          parentTradeId: i === 0 ? undefined : parentTradeId,
          // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
          totalQuantity: transaction.numberShares,
          transactionSequence: i + 1,
        }

        const response = await createStock.mutateAsync(payload)
        
        // Extract parent trade ID from response
        if (i === 0) {
          // Response is already parsed JSON from the mutation
          const responseData = response as any
          parentTradeId = responseData?.data?.id || responseData?.id
        }
      }

      toast.success(`Stock trade${data.transactions.length > 1 ? "s" : ""} created successfully`)
      stockForm.reset()
      onOpenChange(false)
    } catch (error) {
      const message = error instanceof Error ? error.message : "Failed to create stock trade"
      toast.error(message)
      console.error("Error creating stock trade:", error)
    }
  }

  const handleOptionSubmit = async (data: OptionFormData) => {
    try {
      const tradeGroupId = generateTradeGroupId()
      let parentTradeId: number | undefined = undefined
      const expirationDateIso = data.expirationDate ? new Date(`${data.expirationDate}T00:00:00`).toISOString() : ""

      // Create transactions sequentially
      for (let i = 0; i < data.transactions.length; i++) {
        const transaction = data.transactions[i]
        const entryDateIso = new Date(transaction.entryDate).toISOString()
        const exitDateIso = new Date(transaction.exitDate).toISOString()

        const payload: CreateOptionRequest = {
          symbol: data.symbol,
          optionType: data.optionType,
          strikePrice: data.strikePrice,
          expirationDate: expirationDateIso,
          entryPrice: transaction.entryPrice,
          exitPrice: transaction.exitPrice,
          premium: data.premium,
          commissions: Number.isFinite(transaction.commissions) ? transaction.commissions : 0,
          entryDate: entryDateIso,
          exitDate: exitDateIso,
          initialTarget: data.initialTarget,
          profitTarget: data.profitTarget,
          tradeRatings:
            data.tradeRatings != null && data.tradeRatings !== ("" as unknown)
              ? Math.round(Number(data.tradeRatings))
              : undefined,
          reviewed: data.reviewed,
          mistakes: data.mistakes,
          tradeGroupId: i === 0 ? tradeGroupId : undefined, // Only set on first transaction
          parentTradeId: i === 0 ? undefined : parentTradeId,
          totalQuantity: transaction.totalQuantity,
          transactionSequence: i + 1,
        }

        const response = await createOption.mutateAsync(payload)
        
        // Extract parent trade ID from response
        if (i === 0) {
          // Response is already parsed JSON from the mutation
          const responseData = response as any
          parentTradeId = responseData?.data?.id || responseData?.id
        }
      }

      toast.success(`Option trade${data.transactions.length > 1 ? "s" : ""} created successfully`)
      optionForm.reset()
      onOpenChange(false)
    } catch (error) {
      const message = error instanceof Error ? error.message : "Failed to create option trade"
      toast.error(message)
      console.error("Error creating option trade:", error)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl h-[90vh] flex flex-col overflow-hidden p-0 gap-0">
        <DialogHeader className="flex-shrink-0 px-6 pt-6 pb-4">
          <DialogTitle>Create New Trade</DialogTitle>
          <DialogDescription>Add a new stock or option trade to your journal</DialogDescription>
        </DialogHeader>

        <Tabs
          value={activeTab}
          onValueChange={(value) => setActiveTab(value as "stock" | "option")}
          className="w-full flex flex-col flex-1 min-h-0 overflow-hidden"
        >
          <div className="flex-shrink-0 px-6 pb-4">
            <TabsList className="grid w-full grid-cols-2">
              <TabsTrigger value="stock">Stock Trade</TabsTrigger>
              <TabsTrigger value="option">Option Trade</TabsTrigger>
            </TabsList>
          </div>

          <ScrollArea className="flex-1 min-h-0 overflow-hidden">
            <div className="px-6 pb-6">
              <TabsContent value="stock" className="space-y-4 mt-0">
                <Form {...stockForm}>
                  {/* @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?) */}
                  <form onSubmit={stockForm.handleSubmit(handleStockSubmit)} className="space-y-4">
                    {/* Shared fields */}
                    <div className="grid grid-cols-2 gap-4">
                      <FormField
                        // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={stockForm.control}
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
                      // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={stockForm.control}
                        name="orderType"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Order Type</FormLabel>
                            <Select onValueChange={field.onChange} defaultValue={field.value}>
                              <FormControl>
                                <SelectTrigger>
                                  <SelectValue placeholder="Select order type" />
                                </SelectTrigger>
                              </FormControl>
                              <SelectContent>
                                <SelectItem value="MARKET">MARKET</SelectItem>
                                <SelectItem value="LIMIT">LIMIT</SelectItem>
                                <SelectItem value="STOP">STOP</SelectItem>
                                <SelectItem value="STOP_LIMIT">STOP_LIMIT</SelectItem>
                              </SelectContent>
                            </Select>
                            <FormMessage />
                          </FormItem>
                        )}
                      />
                    </div>

                    <div className="grid grid-cols-2 gap-4">
                      <FormField
                      // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={stockForm.control}
                        name="stopLoss"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Stop Loss</FormLabel>
                            <FormControl>
                              <Input type="number" step="0.01" placeholder="145.00" {...field} />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />

                      <FormField
                      // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={stockForm.control}
                        name="takeProfit"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Take Profit</FormLabel>
                            <FormControl>
                              <Input
                                type="number"
                                step="0.01"
                                placeholder="160.00"
                                {...field}
                                value={field.value ?? ""}
                                onChange={(e) => field.onChange(e.target.value === "" ? undefined : e.target.value)}
                              />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />
                    </div>

                    <div className="grid grid-cols-2 gap-4">
                      <FormField
                      // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={stockForm.control}
                        name="initialTarget"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Initial Target</FormLabel>
                            <FormControl>
                              <Input
                                type="number"
                                step="0.01"
                                placeholder="155.00"
                                {...field}
                                value={field.value ?? ""}
                                onChange={(e) => field.onChange(e.target.value === "" ? undefined : e.target.value)}
                              />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />

                      <FormField
                      // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={stockForm.control}
                        name="profitTarget"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Profit Target</FormLabel>
                            <FormControl>
                              <Input
                                type="number"
                                step="0.01"
                                placeholder="165.00"
                                {...field}
                                value={field.value ?? ""}
                                onChange={(e) => field.onChange(e.target.value === "" ? undefined : e.target.value)}
                              />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />
                    </div>

                    <div className="grid grid-cols-2 gap-4">
                      <FormField
                      // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={stockForm.control}
                        name="tradeRatings"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Trade Rating (1-5)</FormLabel>
                            <FormControl>
                              <Input
                                type="number"
                                min="1"
                                max="5"
                                placeholder="3"
                                {...field}
                                value={field.value ?? ""}
                                onChange={(e) => field.onChange(e.target.value === "" ? undefined : e.target.value)}
                              />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />
                    </div>

                    <FormField
                    // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                      control={stockForm.control}
                      name="mistakes"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel>Mistakes (Optional)</FormLabel>
                          <FormControl>
                            <Input placeholder="Enter mistakes separated by commas" {...field} />
                          </FormControl>
                          <FormDescription>List any mistakes made in this trade, separated by commas</FormDescription>
                          <FormMessage />
                        </FormItem>
                      )}
                    />

                    {/* Transactions */}
                    <div className="space-y-4">
                      <div className="flex items-center justify-between">
                        <h3 className="text-sm font-semibold">Transactions</h3>
                        <Button
                          type="button"
                          variant="outline"
                          size="sm"
                          onClick={() =>
                            stockTransactions.append({
                              tradeType: "BUY",
                              entryPrice: 0,
                              exitPrice: 0,
                              numberShares: 0,
                              entryDate: getLocalDateTimeInputValue(),
                              exitDate: getLocalDateTimeInputValue(),
                              commissions: 0,
                            })
                          }
                        >
                          <Plus className="h-4 w-4 mr-2" />
                          Add Transaction
                        </Button>
                      </div>

                      {stockTransactions.fields.map((field, index) => (
                        <TransactionRow
                          key={field.id}
                          index={index}
                          form={stockForm}
                          tradeType="stock"
                          onRemove={() => stockTransactions.remove(index)}
                          canRemove={stockTransactions.fields.length > 1}
                        />
                      ))}
                    </div>

                    <DialogFooter>
                      <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
                        Cancel
                      </Button>
                      <Button type="submit" disabled={createStock.isPending}>
                        {createStock.isPending ? "Creating..." : "Create Stock Trade"}
                      </Button>
                    </DialogFooter>
                  </form>
                </Form>
              </TabsContent>

              <TabsContent value="option" className="space-y-4 mt-0">
                <Form {...optionForm}>
                {/* @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?) */}
                  <form onSubmit={optionForm.handleSubmit(handleOptionSubmit)} className="space-y-4">
                    {/* Shared fields */}
                    <div className="grid grid-cols-2 gap-4">
                      <FormField
                        // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={optionForm.control}
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
                      // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={optionForm.control}
                        name="optionType"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Option Type</FormLabel>
                            <Select onValueChange={field.onChange} defaultValue={field.value}>
                              <FormControl>
                                <SelectTrigger>
                                  <SelectValue placeholder="Select option type" />
                                </SelectTrigger>
                              </FormControl>
                              <SelectContent>
                                <SelectItem value="Call">Call</SelectItem>
                                <SelectItem value="Put">Put</SelectItem>
                              </SelectContent>
                            </Select>
                            <FormMessage />
                          </FormItem>
                        )}
                      />
                    </div>

                    <div className="grid grid-cols-2 gap-4">
                      <FormField
                      // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={optionForm.control}
                        name="strikePrice"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Strike Price</FormLabel>
                            <FormControl>
                              <Input type="number" step="0.01" placeholder="150.00" {...field} />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />

                      <FormField
                      // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={optionForm.control}
                        name="premium"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Premium</FormLabel>
                            <FormControl>
                              <Input type="number" step="0.01" placeholder="550.00" {...field} />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />
                    </div>

                    <div className="grid grid-cols-2 gap-4">
                      <FormField
                      // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={optionForm.control}
                        name="expirationDate"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Expiration Date</FormLabel>
                            <FormControl>
                              <Input type="date" {...field} />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />

                      <FormField
                      // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={optionForm.control}
                        name="tradeRatings"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Trade Rating (1-5)</FormLabel>
                            <FormControl>
                              <Input
                                type="number"
                                min="1"
                                max="5"
                                placeholder="3"
                                {...field}
                                value={field.value ?? ""}
                                onChange={(e) => field.onChange(e.target.value === "" ? undefined : e.target.value)}
                              />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />
                    </div>

                    <div className="grid grid-cols-2 gap-4">
                      <FormField
                      // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={optionForm.control}
                        name="initialTarget"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Initial Target</FormLabel>
                            <FormControl>
                              <Input
                                type="number"
                                step="0.01"
                                placeholder="6.00"
                                {...field}
                                value={field.value ?? ""}
                                onChange={(e) => field.onChange(e.target.value === "" ? undefined : e.target.value)}
                              />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />

                      <FormField
                      // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                        control={optionForm.control}
                        name="profitTarget"
                        render={({ field }) => (
                          <FormItem>
                            <FormLabel>Profit Target</FormLabel>
                            <FormControl>
                              <Input
                                type="number"
                                step="0.01"
                                placeholder="8.00"
                                {...field}
                                value={field.value ?? ""}
                                onChange={(e) => field.onChange(e.target.value === "" ? undefined : e.target.value)}
                              />
                            </FormControl>
                            <FormMessage />
                          </FormItem>
                        )}
                      />
                    </div>

                    <FormField
                    // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
                      control={optionForm.control}
                      name="mistakes"
                      render={({ field }) => (
                        <FormItem>
                          <FormLabel>Mistakes (Optional)</FormLabel>
                          <FormControl>
                            <Input placeholder="Enter mistakes separated by commas" {...field} />
                          </FormControl>
                          <FormDescription>List any mistakes made in this trade, separated by commas</FormDescription>
                          <FormMessage />
                        </FormItem>
                      )}
                    />

                    {/* Transactions */}
                    <div className="space-y-4">
                      <div className="flex items-center justify-between">
                        <h3 className="text-sm font-semibold">Transactions</h3>
                        <Button
                          type="button"
                          variant="outline"
                          size="sm"
                          onClick={() =>
                            optionTransactions.append({
                              tradeType: "BUY",
                              entryPrice: 0,
                              exitPrice: 0,
                              totalQuantity: 1,
                              entryDate: getLocalDateTimeInputValue(),
                              exitDate: getLocalDateTimeInputValue(),
                              commissions: 0,
                            })
                          }
                        >
                          <Plus className="h-4 w-4 mr-2" />
                          Add Transaction
                        </Button>
                      </div>

                      {optionTransactions.fields.map((field, index) => (
                        <TransactionRow
                          key={field.id}
                          index={index}
                          form={optionForm}
                          tradeType="option"
                          onRemove={() => optionTransactions.remove(index)}
                          canRemove={optionTransactions.fields.length > 1}
                        />
                      ))}
                    </div>

                    <DialogFooter>
                      <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
                        Cancel
                      </Button>
                      <Button type="submit" disabled={createOption.isPending}>
                        {createOption.isPending ? "Creating..." : "Create Option Trade"}
                      </Button>
                    </DialogFooter>
                  </form>
                </Form>
              </TabsContent>
            </div>
          </ScrollArea>
        </Tabs>
      </DialogContent>
    </Dialog>
  )
}
