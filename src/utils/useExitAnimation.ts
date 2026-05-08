import { useState, useEffect, useRef, useCallback } from "react";

/**
 * Delays unmounting so CSS exit animations can play.
 *
 * Returns:
 * - `mounted`: whether the DOM element should exist
 * - `animClass`: "entering" on first render, "exiting" when closing, "" otherwise
 * - `onAnimationEnd`: attach to the outermost animated element to unmount after exit
 *
 * Usage:
 *   const { mounted, animClass, onAnimationEnd } = useExitAnimation(open);
 *   if (!mounted) return null;
 *   return <div className={`overlay ${animClass}`} onAnimationEnd={onAnimationEnd}>...
 */
export function useExitAnimation(open: boolean) {
  const [mounted, setMounted] = useState(open);
  const [animClass, setAnimClass] = useState(open ? "entering" : "");
  const prevOpen = useRef(open);

  useEffect(() => {
    if (open && !prevOpen.current) {
      // Opening: mount and mark entering
      setMounted(true);
      setAnimClass("entering");
    } else if (!open && prevOpen.current) {
      // Closing: start exit animation
      setAnimClass("exiting");
    }
    prevOpen.current = open;
  }, [open]);

  // Clear "entering" after first render frame so the entrance animation plays once
  useEffect(() => {
    if (animClass !== "entering") return;
    const id = requestAnimationFrame(() => setAnimClass(""));
    return () => cancelAnimationFrame(id);
  }, [animClass]);

  const onAnimationEnd = useCallback(() => {
    if (!open) {
      setMounted(false);
      setAnimClass("");
    }
  }, [open]);

  return { mounted, animClass, onAnimationEnd };
}
