import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Header } from "@/components/header";

describe("Header", () => {
  const onToggleActivity = vi.fn();

  it("renders greeting", () => {
    render(
      <Header
        greeting="Good morning"
        showActivity={true}
        onToggleActivity={onToggleActivity}
      />,
    );
    expect(screen.getByText("Good morning")).toBeInTheDocument();
  });

  it("renders navigation buttons", () => {
    render(
      <Header
        greeting="Hello"
        showActivity={true}
        onToggleActivity={onToggleActivity}
      />,
    );
    expect(screen.getByTitle("Toggle theme")).toBeInTheDocument();
    expect(screen.getByTitle("Hide activity")).toBeInTheDocument();
  });

  it("shows Show activity button when activity is hidden", () => {
    render(
      <Header
        greeting="Hello"
        showActivity={false}
        onToggleActivity={onToggleActivity}
      />,
    );
    expect(screen.getByTitle("Show activity")).toBeInTheDocument();
  });

  it("toggles theme on button click", async () => {
    render(
      <Header
        greeting="Hello"
        showActivity={true}
        onToggleActivity={onToggleActivity}
      />,
    );
    const user = userEvent.setup();
    await user.click(screen.getByTitle("Toggle theme"));
    expect(document.documentElement.classList.contains("dark")).toBe(true);
  });

  it("calls onToggleActivity when activity button clicked", async () => {
    render(
      <Header
        greeting="Hello"
        showActivity={true}
        onToggleActivity={onToggleActivity}
      />,
    );
    const user = userEvent.setup();
    await user.click(screen.getByTitle("Hide activity"));
    expect(onToggleActivity).toHaveBeenCalledOnce();
  });
});
