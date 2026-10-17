"use client";

import * as React from "react";
import { ChevronDown } from "lucide-react";

import { cn } from "@/lib/utils";

/**
 * Lightweight select — a styled native <select> to keep the bundle small.
 * Behaviour mirrors shadcn's Select API so existing call sites can use
 * SelectTrigger / SelectValue / SelectContent / SelectItem unchanged.
 */
export const Select = React.forwardRef<
  HTMLSelectElement,
  React.SelectHTMLAttributes<HTMLSelectElement> & {
    onValueChange?: (value: string) => void;
    value?: string;
  }
>(({ className, onValueChange, value, children, ...rest }, ref) => {
  return (
    <div className="relative">
      <select
        ref={ref}
        value={value}
        onChange={(e) => onValueChange?.(e.target.value)}
        className={cn(
          "flex h-9 w-full appearance-none rounded-md border border-input bg-background px-3 py-1 pr-8 text-sm shadow-sm",
          "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1",
          "disabled:cursor-not-allowed disabled:opacity-50",
          className
        )}
        {...rest}
      >
        {children}
      </select>
      <ChevronDown className="pointer-events-none absolute right-2 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
    </div>
  );
});
Select.displayName = "Select";

export const SelectTrigger = Select;

export const SelectValue: React.FC<{ placeholder?: string }> = ({
  placeholder,
}) => <option value="">{placeholder ?? ""}</option>;

export const SelectContent: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => <>{children}</>;

export const SelectItem: React.FC<{ value: string; children: React.ReactNode }> = ({
  value,
  children,
}) => <option value={value}>{children}</option>;
