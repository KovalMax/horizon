import type { ReactNode } from "react";

export function Icon({ name, size = 18 }: { name: string; size?: number }) {
  const common = {
    width: size,
    height: size,
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    strokeWidth: 1.7,
    strokeLinecap: "round" as const,
    strokeLinejoin: "round" as const,
    "aria-hidden": true as const,
  };
  const paths: Record<string, ReactNode> = {
    layers: (
      <>
        <path d="m12 3 9 5-9 5-9-5 9-5Z" />
        <path d="m3 12 9 5 9-5M3 16l9 5 9-5" />
      </>
    ),
    bell: (
      <>
        <path d="M18 8a6 6 0 0 0-12 0c0 7-3 7-3 9h18c0-2-3-2-3-9" />
        <path d="M10 21h4" />
      </>
    ),
    settings: (
      <>
        <circle cx="12" cy="12" r="3" />
        <path
          d="m19.4 15 .1.1 1.4 1.1-1.4 2.4-1.7-.7a8 8 0 0 1-1.7 1l-.3 1.8h-2.8l-.3-1.8a8 8 0 0 1-1.7-1l-1.7.7-1.4-2.4L7.3 15a8 8 0 0 1 0-2l-1.4-1.1 1.4-2.4 1.7.7a8 8 0 0 1 1.7-1l.3-1.8h2.8l.3 1.8a8 8 0 0 1 1.7 1l1.7-.7 1.4 2.4-1.4 1.1a8 8 0 0 1-.1 2Z"
          transform="translate(0 -1) scale(.9) translate(1.3 1.3)"
        />
      </>
    ),
    person: (
      <>
        <circle cx="12" cy="8" r="3.5" />
        <path d="M5 21v-2a7 7 0 0 1 14 0v2" />
      </>
    ),
    home: (
      <>
        <path d="m3 10 9-7 9 7v10a1 1 0 0 1-1 1h-5v-7H9v7H4a1 1 0 0 1-1-1V10Z" />
      </>
    ),
    plane: (
      <>
        <path d="m3 11 18-8-8 18-2-8-8-2Z" />
        <path d="m11 13 5-5" />
      </>
    ),
    calendar: (
      <>
        <rect x="3" y="5" width="18" height="16" rx="2" />
        <path d="M16 3v4M8 3v4M3 10h18" />
      </>
    ),
    list: (
      <>
        <path d="M9 6h11M9 12h11M9 18h11" />
        <path d="m3 6 1 1 2-2M3 12l1 1 2-2M3 18l1 1 2-2" />
      </>
    ),
    pin: (
      <>
        <path d="M20 10c0 5-8 12-8 12S4 15 4 10a8 8 0 1 1 16 0Z" />
        <circle cx="12" cy="10" r="2.5" />
      </>
    ),
    map: (
      <>
        <path d="m3 6 6-3 6 3 6-3v15l-6 3-6-3-6 3V6Z" />
        <path d="M9 3v15M15 6v15" />
      </>
    ),
    users: (
      <>
        <path d="M16 21v-2a4 4 0 0 0-4-4H7a4 4 0 0 0-4 4v2" />
        <circle cx="9.5" cy="7" r="4" />
        <path d="M20 21v-2a4 4 0 0 0-3-3.9M16 3.1a4 4 0 0 1 0 7.8" />
      </>
    ),
    plus: (
      <>
        <path d="M12 5v14M5 12h14" />
      </>
    ),
    arrow: (
      <>
        <path d="M5 12h14M13 6l6 6-6 6" />
      </>
    ),
    clock: (
      <>
        <circle cx="12" cy="12" r="9" />
        <path d="M12 7v5l3 2" />
      </>
    ),
    check: (
      <>
        <path d="m5 12 4 4L19 6" />
      </>
    ),
    more: (
      <>
        <circle cx="5" cy="12" r="1" />
        <circle cx="12" cy="12" r="1" />
        <circle cx="19" cy="12" r="1" />
      </>
    ),
    menu: (
      <>
        <path d="M4 6h16M4 12h16M4 18h16" />
      </>
    ),
    "chevron-down": <path d="m6 9 6 6 6-6" />,
    "chevron-up": <path d="m6 15 6-6 6 6" />,
    repeat: (
      <>
        <path d="m17 2 4 4-4 4" />
        <path d="M3 11V9a3 3 0 0 1 3-3h15M7 22l-4-4 4-4" />
        <path d="M21 13v2a3 3 0 0 1-3 3H3" />
      </>
    ),
    globe: (
      <>
        <circle cx="12" cy="12" r="9" />
        <path d="M3 12h18M12 3a14 14 0 0 1 0 18M12 3a14 14 0 0 0 0 18" />
      </>
    ),
    flask: (
      <>
        <path d="M9 3h6M10 3v6.2l-5.6 9.1A1.8 1.8 0 0 0 5.9 21h12.2a1.8 1.8 0 0 0 1.5-2.7L14 9.2V3" />
        <path d="M8 15h8" />
      </>
    ),
    wifi: (
      <>
        <path d="M5 9a11 11 0 0 1 14 0M8 12a6.5 6.5 0 0 1 8 0M11 15a2 2 0 0 1 2 0M3 3l18 18" />
      </>
    ),
  };
  return <svg {...common}>{paths[name] ?? paths.layers}</svg>;
}

export function HorizonMark({ large = false }: { large?: boolean }) {
  return (
    <svg
      aria-hidden="true"
      className={large ? "horizon-mark large" : "horizon-mark"}
      viewBox="0 0 32 32"
      fill="none"
    >
      <rect width="32" height="32" rx="9" fill="var(--primary)" />
      <path d="M9 19a7 7 0 0 1 14 0Z" fill="var(--canvas)" />
      <path
        d="M16 8.5v2M9.2 11.3l1.4 1.4M22.8 11.3l-1.4 1.4"
        stroke="var(--canvas)"
        strokeWidth="1.6"
        strokeLinecap="round"
        opacity=".75"
      />
      <path
        d="M6.5 19h19"
        stroke="var(--canvas)"
        strokeWidth="1.8"
        strokeLinecap="round"
      />
      <path
        d="M10.5 23h11"
        stroke="var(--canvas)"
        strokeWidth="1.6"
        strokeLinecap="round"
        opacity=".55"
      />
    </svg>
  );
}

export function AppLink({
  to,
  children,
  className = "",
  onNavigate,
  ariaLabel,
}: {
  to: string;
  children: ReactNode;
  className?: string;
  onNavigate: (to: string) => void;
  ariaLabel?: string;
}) {
  return (
    <a
      href={to}
      className={className}
      aria-label={ariaLabel}
      onClick={(e) => {
        if (!e.metaKey && !e.ctrlKey && e.button === 0) {
          e.preventDefault();
          onNavigate(to);
        }
      }}
    >
      {children}
    </a>
  );
}
export function Button({
  children,
  onClick,
  variant = "primary",
  disabled = false,
  className = "",
  type = "button",
  "aria-label": ariaLabel,
}: {
  children: ReactNode;
  onClick?: () => void;
  variant?: "primary" | "outline" | "quiet";
  disabled?: boolean;
  className?: string;
  type?: "button" | "submit" | "reset";
  "aria-label"?: string;
}) {
  return (
    <button
      className={`button ${variant} ${className}`}
      type={type}
      onClick={onClick}
      disabled={disabled}
      aria-label={ariaLabel}
    >
      {children}
    </button>
  );
}

type SpaceSummary = {
  id: string;
  name: string;
  tone: string;
  icon: string;
};

export function SpaceGlyph({
  space,
  small = false,
}: {
  space?: Pick<SpaceSummary, "tone" | "icon">;
  small?: boolean;
}) {
  return (
    <span
      className={`space-glyph ${space?.tone ?? "all"} ${small ? "small" : ""}`}
    >
      <Icon name={space?.icon ?? "layers"} size={small ? 13 : 18} />
    </span>
  );
}
export function SpacePill({ space }: { space: SpaceSummary }) {
  return (
    <span className={`space-pill ${space.tone}`}>
      <Icon name={space.icon} size={12} />
      {space.name}
    </span>
  );
}
export function BottomBar({
  path,
  space,
  go,
  goSpace,
  onMore,
}: {
  path: string;
  space: SpaceSummary;
  go: (to: string) => void;
  goSpace: (id: string, section?: string) => void;
  onMore: () => void;
}) {
  const isHome = path === "/";
  const isSpacePage = path.startsWith("/spaces/");
  const current = path.split("/")[3] ?? "calendar";
  return (
    <nav className="bottom-nav" aria-label="Primary">
      <button className={isHome ? "active" : ""} onClick={() => go("/")}>
        <Icon name="layers" />
        All
      </button>
      <button
        className={isSpacePage && current === "calendar" ? "active" : ""}
        onClick={() => goSpace(space.id, "calendar")}
      >
        <Icon name="calendar" />
        Calendar
      </button>
      <button
        className={isSpacePage && current === "tasks" ? "active" : ""}
        onClick={() => goSpace(space.id, "tasks")}
      >
        <Icon name="list" />
        To-dos
      </button>
      <button
        className={isSpacePage && current === "places" ? "active" : ""}
        onClick={() => goSpace(space.id, "places")}
      >
        <Icon name="pin" />
        Places
      </button>
      <button onClick={onMore} aria-haspopup="dialog">
        <Icon name="more" />
        More
      </button>
    </nav>
  );
}
