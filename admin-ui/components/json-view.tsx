"use client";

interface JsonViewProps {
  data: unknown;
  className?: string;
}

export function JsonView({ data, className = "" }: JsonViewProps) {
  const jsonString = JSON.stringify(data, null, 2);

  // Simple JSON syntax highlighter
  const highlighted = jsonString
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(
      /("(\\u[a-zA-Z0-9]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(?:\.\d*)?(?:[eE][+\-]?\d+)?)/g,
      (match) => {
        let cls = "text-foreground";
        if (/^"/.test(match)) {
          if (/:$/.test(match)) {
            // JSON key
            cls = "text-blue-600 dark:text-blue-400";
          } else {
            // String value
            cls = "text-green-600 dark:text-green-400";
          }
        } else if (/true|false/.test(match)) {
          // Boolean
          cls = "text-purple-600 dark:text-purple-400";
        } else if (/null/.test(match)) {
          // Null
          cls = "text-red-600 dark:text-red-400";
        } else {
          // Number
          cls = "text-orange-600 dark:text-orange-400";
        }
        return `<span class="${cls}">${match}</span>`;
      }
    );

  return (
    <pre
      className={`max-h-48 overflow-auto whitespace-pre-wrap rounded bg-muted/50 p-2 font-mono text-[10px] ${className}`}
      dangerouslySetInnerHTML={{ __html: highlighted }}
    />
  );
}
