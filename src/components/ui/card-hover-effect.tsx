import { AnimatePresence, motion } from "framer-motion";
import { useState } from "react";
import { cn } from "../../lib/utils";

export interface HoverItem {
  key: string;
  content: React.ReactNode;
  onClick?: () => void;
  disabled?: boolean;
}

/**
 * Aceternity-style card hover effect adapted for light-mode React/Electron.
 * Renders a grid where a smooth animated background slides between hovered cards.
 */
export const HoverEffect = ({
  items,
  className,
}: {
  items: HoverItem[];
  className?: string;
}) => {
  const [hoveredIndex, setHoveredIndex] = useState<number | null>(null);

  return (
    <div className={cn("grid grid-cols-3 gap-4", className)}>
      {items.map((item, idx) => (
        <div
          key={item.key}
          className={cn(
            "relative group block p-0 h-full w-full",
            item.disabled ? "cursor-not-allowed opacity-50" : "cursor-pointer"
          )}
          onMouseEnter={() => !item.disabled && setHoveredIndex(idx)}
          onMouseLeave={() => setHoveredIndex(null)}
          onClick={item.disabled ? undefined : item.onClick}
        >
          {/* Animated background highlight */}
          <AnimatePresence>
            {hoveredIndex === idx && !item.disabled && (
              <motion.span
                className="absolute inset-0 h-full w-full block rounded-2xl"
                style={{ background: 'var(--accent-subtle)' }}
                layoutId="hoverBackground"
                initial={{ opacity: 0 }}
                animate={{
                  opacity: 1,
                  transition: { duration: 0.15 },
                }}
                exit={{
                  opacity: 0,
                  transition: { duration: 0.15, delay: 0.2 },
                }}
              />
            )}
          </AnimatePresence>

          {/* Card content sits above the animated background */}
          <div className="relative z-10 h-full">{item.content}</div>
        </div>
      ))}
    </div>
  );
};
