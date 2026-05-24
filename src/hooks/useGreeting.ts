const hour = new Date().getHours();
export const GREETING =
  hour < 12 ? "Good morning" : hour < 18 ? "Good afternoon" : "Good evening";
