import * as React from "react"
import { Provider, Root, Trigger, Portal, Content, Arrow } from "@radix-ui/react-tooltip"

import { cn } from "@/lib/utils"

function TooltipProvider({ delayDuration = 300, ...props }: React.ComponentProps<typeof Provider>) {
  return <Provider delayDuration={delayDuration} {...props} />
}

function Tooltip({ ...props }: React.ComponentProps<typeof Root>) {
  return <Root {...props} />
}

function TooltipTrigger({ ...props }: React.ComponentProps<typeof Trigger>) {
  return <Trigger {...props} />
}

function TooltipContent({
  className,
  sideOffset = 4,
  children,
  ...props
}: React.ComponentProps<typeof Content>) {
  return (
    <Portal>
      <Content
        sideOffset={sideOffset}
        className={cn(
          "z-50 max-w-64 rounded-lg border bg-popover px-3 py-1.5 text-xs text-popover-foreground shadow-md animate-in fade-in-0 zoom-in-95 data-side=bottom:slide-in-from-top-1 data-side=left:slide-in-from-right-1 data-side=right:slide-in-from-left-1 data-side=top:slide-in-from-bottom-1",
          className
        )}
        {...props}
      >
        {children}
        <Arrow className="fill-popover" />
      </Content>
    </Portal>
  )
}

export { TooltipProvider, Tooltip, TooltipTrigger, TooltipContent }
