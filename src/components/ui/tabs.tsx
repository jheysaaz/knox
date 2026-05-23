import * as React from "react";
import { Root, List, Trigger, Content } from "@radix-ui/react-tabs";

import { cn } from "@/lib/utils";

function Tabs({ className, ...props }: React.ComponentProps<typeof Root>) {
  return (
    <Root
      data-slot="tabs"
      className={cn("flex flex-col gap-3 w-full", className)}
      {...props}
    />
  );
}

function TabsList({ className, ...props }: React.ComponentProps<typeof List>) {
  return (
    <List
      data-slot="tabs-list"
      className={cn(
        "inline-flex w-fit items-center gap-1 rounded-lg bg-muted p-0.5",
        className,
      )}
      {...props}
    />
  );
}

function TabsTrigger({
  className,
  ...props
}: React.ComponentProps<typeof Trigger>) {
  return (
    <Trigger
      data-slot="tabs-trigger"
      className={cn(
        "inline-flex items-center justify-center gap-1.5 rounded-md px-2.5 py-1 text-sm font-medium whitespace-nowrap transition-all outline-none hover:text-foreground focus-visible:ring-3 focus-visible:ring-ring/50 focus-visible:border-ring disabled:pointer-events-none disabled:opacity-50 data-[state=active]:bg-background data-[state=active]:text-foreground data-[state=active]:shadow-xs",
        className,
      )}
      {...props}
    />
  );
}

function TabsContent({
  className,
  ...props
}: React.ComponentProps<typeof Content>) {
  return (
    <Content
      data-slot="tabs-content"
      className={cn("flex-1 outline-none", className)}
      {...props}
    />
  );
}

export { Tabs, TabsList, TabsTrigger, TabsContent };
