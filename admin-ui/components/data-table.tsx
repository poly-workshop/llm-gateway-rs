import { cn } from "@/lib/utils";

export function DataTable({
  className,
  children,
}: {
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <div className={cn("ring-foreground/10 overflow-hidden rounded-none ring-1", className)}>
      <table className="w-full text-xs">
        {children}
      </table>
    </div>
  );
}

export function DataTableHeader({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <thead className="bg-muted/50 border-b">
      <tr>{children}</tr>
    </thead>
  );
}

export function DataTableHead({
  className,
  children,
}: {
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <th
      className={cn(
        "px-3 py-2 text-left text-xs font-medium text-muted-foreground",
        className
      )}
    >
      {children}
    </th>
  );
}

export function DataTableBody({
  children,
}: {
  children: React.ReactNode;
}) {
  return <tbody className="divide-y">{children}</tbody>;
}

export function DataTableRow({
  className,
  children,
}: {
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <tr className={cn("hover:bg-muted/30 transition-colors", className)}>
      {children}
    </tr>
  );
}

export function DataTableCell({
  className,
  children,
}: {
  className?: string;
  children: React.ReactNode;
}) {
  return <td className={cn("px-3 py-2", className)}>{children}</td>;
}

export function DataTableEmpty({ message }: { message?: string }) {
  return (
    <tr>
      <td
        colSpan={100}
        className="px-3 py-8 text-center text-xs text-muted-foreground"
      >
        {message || "No data"}
      </td>
    </tr>
  );
}
