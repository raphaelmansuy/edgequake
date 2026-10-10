import { describe, expect, it } from "vitest";
import { connectionLocality } from "@/components/settings/connection-locality";

describe("connectionLocality", () => {
  it("marks loopback and RFC1918 hosts as local with private allowed", () => {
    expect(connectionLocality("http://127.0.0.1:9050")).toEqual({
      locality: "local",
      allowPrivate: true,
    });
    expect(connectionLocality("http://localhost:11434")).toEqual({
      locality: "local",
      allowPrivate: true,
    });
    expect(connectionLocality("http://10.0.0.2:8080")).toEqual({
      locality: "local",
      allowPrivate: true,
    });
    expect(connectionLocality("http://192.168.1.10")).toEqual({
      locality: "local",
      allowPrivate: true,
    });
  });

  it("marks public hosts as cloud without private network", () => {
    expect(connectionLocality("https://api.openai.com/v1")).toEqual({
      locality: "cloud",
      allowPrivate: false,
    });
  });
});
