import * as React from "react"
import { Root, Item, Indicator } from "@radix-ui/react-radio-group"

import { cn } from "@/lib/utils"
import { CheckIcon } from "lucide-react"

function RadioGroup({
  className,
  ...props
}: React.ComponentProps<typeof Root>) {
  return (
    <Root
      data-slot="radio-group"
      className={cn("grid gap-2", className)}
      {...props}
    />
  )
}

function RadioGroupItem({
  className,
  children,
  ...props
}: React.ComponentProps<typeof Item>) {
  return (
    <Item
      data-slot="radio-group-item"
      className={cn(
        "flex items-center gap-2 rounded-lg border border-input px-3 py-2 text-sm transition-colors outline-none hover:bg-accent hover:text-accent-foreground focus-visible:border-ring focus-visible:ring-3 focus-visible:ring-ring/50 data-disabled:cursor-not-allowed data-disabled:opacity-50 data-[state=checked]:border-primary data-[state=checked]:bg-primary/5",
        className
      )}
      {...props}
    >
      <span className="flex size-4 shrink-0 items-center justify-center rounded-full border border-input data-[state=checked]:border-primary data-[state=checked]:bg-primary">
        <Indicator>
          <CheckIcon className="size-3 text-primary-foreground" />
        </Indicator>
      </span>
      {children}
    </Item>
  )
}

export { RadioGroup, RadioGroupItem }
