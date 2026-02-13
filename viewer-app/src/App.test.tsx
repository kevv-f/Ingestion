import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, act } from "@testing-library/react";
import App from "./App";

// Mock the Tauri API
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

import { invoke } from "@tauri-apps/api/core";

const mockInvoke = vi.mocked(invoke);

describe("App", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it("renders the app title", async () => {
    // Mock the API to return empty sources
    mockInvoke.mockResolvedValue({
      items: [],
      total: 0,
      page: 1,
      page_size: 50,
      has_more: false,
    });

    await act(async () => {
      render(<App />);
    });

    // Wait for the component to finish loading
    await waitFor(() => {
      expect(screen.getByTestId("app-title")).toBeInTheDocument();
    });

    expect(screen.getByText("Content Viewer")).toBeInTheDocument();
  });

  it("renders the subtitle", async () => {
    // Mock the API to return empty sources
    mockInvoke.mockResolvedValue({
      items: [],
      total: 0,
      page: 1,
      page_size: 50,
      has_more: false,
    });

    await act(async () => {
      render(<App />);
    });

    // Wait for the component to finish loading
    await waitFor(() => {
      expect(screen.getByText("Browse your ingested content")).toBeInTheDocument();
    });
  });

  it("shows loading state initially", async () => {
    // Mock the API to delay response
    mockInvoke.mockImplementation(() => new Promise(() => {})); // Never resolves

    await act(async () => {
      render(<App />);
    });

    expect(screen.getByTestId("app-loading")).toBeInTheDocument();
    expect(screen.getByText("Loading content sources...")).toBeInTheDocument();
  });

  it("shows empty state when no content sources exist", async () => {
    // Mock the API to return empty sources
    mockInvoke.mockResolvedValue({
      items: [],
      total: 0,
      page: 1,
      page_size: 50,
      has_more: false,
    });

    await act(async () => {
      render(<App />);
    });

    // Wait for the empty state to appear
    await waitFor(() => {
      expect(screen.getByTestId("empty-state")).toBeInTheDocument();
    });
  });

  it("shows error state when API call fails", async () => {
    // Mock the API to reject
    mockInvoke.mockRejectedValue(new Error("Database connection failed"));

    await act(async () => {
      render(<App />);
    });

    // Wait for the error state to appear
    await waitFor(() => {
      expect(screen.getByTestId("error-state")).toBeInTheDocument();
    });

    expect(screen.getByText("Failed to load content sources")).toBeInTheDocument();
  });

  it("shows grid view when content sources exist", async () => {
    // Mock the API to return some sources
    mockInvoke.mockResolvedValue({
      items: [
        {
          id: 1,
          source_type: "slack",
          source_path: "https://slack.com/test",
          ehl_doc_id: "doc-1",
          chunk_count: 3,
          created_at: "2024-01-01T00:00:00Z",
          updated_at: "2024-01-01T00:00:00Z",
          title: "Test Document",
          preview_text: "This is a test preview",
        },
      ],
      total: 1,
      page: 1,
      page_size: 50,
      has_more: false,
    });

    await act(async () => {
      render(<App />);
    });

    // Wait for the grid view to appear
    await waitFor(() => {
      expect(screen.getByTestId("grid-view")).toBeInTheDocument();
    });

    // Check that the content card is rendered
    expect(screen.getByText("Test Document")).toBeInTheDocument();
  });

  it("displays content count in header", async () => {
    // Mock the API to return some sources
    mockInvoke.mockResolvedValue({
      items: [
        {
          id: 1,
          source_type: "slack",
          source_path: "https://slack.com/test",
          ehl_doc_id: "doc-1",
          chunk_count: 3,
          created_at: "2024-01-01T00:00:00Z",
          updated_at: "2024-01-01T00:00:00Z",
          title: "Test Document",
          preview_text: "This is a test preview",
        },
      ],
      total: 1,
      page: 1,
      page_size: 50,
      has_more: false,
    });

    await act(async () => {
      render(<App />);
    });

    // Wait for the count to appear
    await waitFor(() => {
      expect(screen.getByTestId("content-count")).toBeInTheDocument();
    });

    expect(screen.getByText("1 item")).toBeInTheDocument();
  });
});
