import type { ButtonHTMLAttributes } from "react";
export function IconButton({
  title,
  children,
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & { title: string }) {
  return (
    <button className="icon-button" aria-label={title} title={title} {...props}>
      {children}
    </button>
  );
}
