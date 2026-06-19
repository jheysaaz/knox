import { describe, it, expect, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Header } from "@/components/header";

describe("Header", () => {
  const onToggleActivity = vi.fn();
  const onToggleHistory = vi.fn();

  it("renders greeting", () => {
    render(
      <Header
        greeting="Good morning"
        showActivity={true}
        onToggleActivity={onToggleActivity}
        showHistory={false}
        onToggleHistory={onToggleHistory}
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
        showHistory={false}
        onToggleHistory={onToggleHistory}
      />,
    );
    expect(screen.getByTitle("Toggle theme")).toBeInTheDocument();
    expect(screen.getByTitle("Hide activity")).toBeInTheDocument();
    expect(screen.getByTitle("Show history")).toBeInTheDocument();
  });

  it("shows Show activity button when activity is hidden", () => {
    render(
      <Header
        greeting="Hello"
        showActivity={false}
        onToggleActivity={onToggleActivity}
        showHistory={false}
        onToggleHistory={onToggleHistory}
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
        showHistory={false}
        onToggleHistory={onToggleHistory}
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
        showHistory={false}
        onToggleHistory={onToggleHistory}
      />,
    );
    const user = userEvent.setup();
    await user.click(screen.getByTitle("Hide activity"));
    expect(onToggleActivity).toHaveBeenCalledOnce();
  });

  it("calls onToggleHistory when history button clicked", async () => {
    render(
      <Header
        greeting="Hello"
        showActivity={true}
        onToggleActivity={onToggleActivity}
        showHistory={false}
        onToggleHistory={onToggleHistory}
      />,
    );
    const user = userEvent.setup();
    await user.click(screen.getByTitle("Show history"));
    expect(onToggleHistory).toHaveBeenCalledOnce();
  });

  it("shows Hide history button when history is shown", () => {
    render(
      <Header
        greeting="Hello"
        showActivity={true}
        onToggleActivity={onToggleActivity}
        showHistory={true}
        onToggleHistory={onToggleHistory}
      />,
    );
    expect(screen.getByTitle("Hide history")).toBeInTheDocument();
  });


});
