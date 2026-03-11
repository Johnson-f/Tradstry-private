import * as React from "react"
import { cva, type VariantProps } from "class-variance-authority"
import { cn } from "@/lib/utils"

const financeTabVariants = cva(
  "inline-flex items-center justify-center whitespace-nowrap rounded-full px-6 py-2.5 text-sm font-medium transition-all duration-200 ease-in-out border border-transparent hover:border-gray-600 focus:outline-none focus:ring-2 focus:ring-cyan-500/20",
  {
    variants: {
      variant: {
        active: "bg-transparent border-cyan-400 text-cyan-400 shadow-sm",
        inactive: "bg-transparent border-gray-700 text-gray-400 hover:text-gray-300 hover:border-gray-500",
      },
    },
    defaultVariants: {
      variant: "inactive",
    },
  },
)

interface FinanceTabButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement>,
    VariantProps<typeof financeTabVariants> {
  active?: boolean
}

const FinanceTabButton = React.forwardRef<HTMLButtonElement, FinanceTabButtonProps>(
  ({ className, active, children, ...props }, ref) => {
    const buttonVariant = active ? "active" : "inactive"

    return (
      <button className={cn(financeTabVariants({ variant: buttonVariant, className }))} ref={ref} {...props}>
        {children}
      </button>
    )
  },
)

FinanceTabButton.displayName = "FinanceTabButton"

export { FinanceTabButton, financeTabVariants }
