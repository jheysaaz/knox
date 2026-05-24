import { useState, useEffect } from "react";

const getGreeting = () => {
  const hour = new Date().getHours();
  if (hour < 12) return "Good morning";
  if (hour < 18) return "Good afternoon";
  return "Good evening";
};

export function useGreeting() {
  const [greeting, setGreeting] = useState(getGreeting);

  useEffect(() => {
    const msUntilNext = () => {
      const now = new Date();
      const hour = now.getHours();
      let next = new Date(now);
      if (hour < 12) next.setHours(12, 0, 0, 0);
      else if (hour < 18) next.setHours(18, 0, 0, 0);
      else {
        next.setDate(next.getDate() + 1);
        next.setHours(0, 0, 0, 0);
      }
      return next.getTime() - now.getTime();
    };

    const ref = { current: 0 };
    const schedule = () => {
      ref.current = window.setTimeout(() => {
        setGreeting(getGreeting());
        schedule();
      }, msUntilNext());
    };
    schedule();
    return () => clearTimeout(ref.current);
  }, []);

  return greeting;
}
