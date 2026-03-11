"use client"

import React from "react"
import { FormControl, FormField, FormItem, FormLabel, FormMessage } from "@/components/ui/form"
import { Input } from "@/components/ui/input"
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"
import { Button } from "@/components/ui/button"
import { Trash2 } from "lucide-react"
import { UseFormReturn } from "react-hook-form"

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

interface TransactionRowProps {
  index: number
  form: UseFormReturn<any>
  tradeType: "stock" | "option"
  onRemove: () => void
  canRemove: boolean
}

export function TransactionRow({ index, form, tradeType, onRemove, canRemove }: TransactionRowProps) {
  const quantityFieldName = tradeType === "stock" ? `transactions.${index}.numberShares` : `transactions.${index}.totalQuantity`

  return (
    <div className="border rounded-lg p-4 space-y-4 bg-muted/30">
      <div className="flex items-center justify-between mb-2">
        <h4 className="text-sm font-semibold">Transaction {index + 1}</h4>
        {canRemove && (
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={onRemove}
            className="h-8 w-8 text-destructive hover:text-destructive"
          >
            <Trash2 className="h-4 w-4" />
          </Button>
        )}
      </div>

      <div className="grid grid-cols-2 gap-4">
        <FormField
          control={form.control}
          name={`transactions.${index}.tradeType`}
          render={({ field }) => (
            <FormItem>
              <FormLabel>Trade Type</FormLabel>
              <Select onValueChange={field.onChange} defaultValue={field.value || "BUY"}>
                <FormControl>
                  <SelectTrigger>
                    <SelectValue placeholder="Select trade type" />
                  </SelectTrigger>
                </FormControl>
                <SelectContent>
                  <SelectItem value="BUY">BUY</SelectItem>
                  <SelectItem value="SELL">SELL</SelectItem>
                </SelectContent>
              </Select>
              <FormMessage />
            </FormItem>
          )}
        />

        <FormField
          control={form.control}
          name={quantityFieldName}
          render={({ field }) => (
            <FormItem>
              <FormLabel>{tradeType === "stock" ? "Number of Shares" : "Number of Contracts"}</FormLabel>
              <FormControl>
                <Input
                  type="number"
                  step={tradeType === "stock" ? "0.01" : "1"}
                  placeholder={tradeType === "stock" ? "100" : "1"}
                  {...field}
                  value={field.value ?? ""}
                  onChange={(e) => field.onChange(e.target.value === "" ? undefined : parseFloat(e.target.value))}
                />
              </FormControl>
              <FormMessage />
            </FormItem>
          )}
        />
      </div>

      <div className="grid grid-cols-2 gap-4">
        <FormField
          control={form.control}
          name={`transactions.${index}.entryPrice`}
          render={({ field }) => (
            <FormItem>
              <FormLabel>Entry Price</FormLabel>
              <FormControl>
                <Input
                  type="number"
                  step="0.01"
                  placeholder={tradeType === "stock" ? "150.00" : "5.50"}
                  {...field}
                  value={field.value ?? ""}
                  onChange={(e) => field.onChange(e.target.value === "" ? undefined : parseFloat(e.target.value))}
                />
              </FormControl>
              <FormMessage />
            </FormItem>
          )}
        />

        <FormField
          control={form.control}
          name={`transactions.${index}.exitPrice`}
          render={({ field }) => (
            <FormItem>
              <FormLabel>Exit Price</FormLabel>
              <FormControl>
                <Input
                  type="number"
                  step="0.01"
                  placeholder={tradeType === "stock" ? "155.00" : "6.50"}
                  {...field}
                  value={field.value ?? 0}
                  onChange={(e) => field.onChange(e.target.value === "" ? 0 : parseFloat(e.target.value))}
                />
              </FormControl>
              <FormMessage />
            </FormItem>
          )}
        />
      </div>

      <div className="grid grid-cols-2 gap-4">
        <FormField
          control={form.control}
          name={`transactions.${index}.entryDate`}
          render={({ field }) => (
            <FormItem>
              <FormLabel>Entry Date</FormLabel>
              <FormControl>
                <Input
                  type="datetime-local"
                  {...field}
                  value={field.value || getLocalDateTimeInputValue()}
                />
              </FormControl>
              <FormMessage />
            </FormItem>
          )}
        />

        <FormField
          control={form.control}
          name={`transactions.${index}.exitDate`}
          render={({ field }) => (
            <FormItem>
              <FormLabel>Exit Date</FormLabel>
              <FormControl>
                <Input
                  type="datetime-local"
                  {...field}
                  value={field.value || getLocalDateTimeInputValue()}
                  onChange={(e) => field.onChange(e.target.value === "" ? getLocalDateTimeInputValue() : e.target.value)}
                />
              </FormControl>
              <FormMessage />
            </FormItem>
          )}
        />
      </div>

      <FormField
        control={form.control}
        name={`transactions.${index}.commissions`}
        render={({ field }) => (
          <FormItem>
            <FormLabel>Commissions</FormLabel>
            <FormControl>
              <Input
                type="number"
                step="0.01"
                placeholder="0.00"
                {...field}
                value={field.value ?? ""}
                onChange={(e) => field.onChange(e.target.value === "" ? 0 : parseFloat(e.target.value))}
              />
            </FormControl>
            <FormMessage />
          </FormItem>
        )}
      />
    </div>
  )
}
