export type Space = {
  id: string;
  name: string;
  role: "Owner" | "Editor" | "Viewer";
  members: number;
  tone: string;
  icon: string;
};
export type EventItem = {
  id: number | string;
  title: string;
  space: string;
  day: number;
  time?: string;
  end?: string;
  allDay?: boolean;
  place?: string;
  repeat?: string;
  zone?: string;
  notes?: string;
  spaceName?: string;
  spaceTone?: string;
  spaceIcon?: string;
  startDate?: string;
  startsAt?: string;
  endsAt?: string;
  timeZone?: string;
  occurrenceDate?: string;
  occurrenceId?: string;
  isOccurrenceOverride?: boolean;
  recurrenceEndsOn?: string;
};
export type TaskItem = {
  id: number | string;
  title: string;
  space: string;
  day?: number;
  time?: string;
  owner?: string;
  assigneeId?: string;
  reminders?: number;
  reminderOffsetsMinutes?: number[];
  myReminderOffsetsMinutes?: number[];
  done?: boolean;
  list: string;
  listId?: string;
  spaceName?: string;
  spaceTone?: string;
  spaceIcon?: string;
};
export type PlaceItem = {
  id?: string;
  name: string;
  category: string;
  address: string;
  note?: string;
  space: string;
  spaceName?: string;
  spaceTone?: string;
};

export const spaces: Space[] = [
  {
    id: "personal",
    name: "Maya's Space",
    role: "Owner",
    members: 1,
    tone: "personal",
    icon: "person",
  },
  {
    id: "home",
    name: "Linden Street Home",
    role: "Owner",
    members: 3,
    tone: "home",
    icon: "home",
  },
  {
    id: "trip",
    name: "Lisbon long weekend",
    role: "Editor",
    members: 4,
    tone: "trip",
    icon: "plane",
  },
];
export const dateAt = (days: number) => {
  const d = new Date();
  d.setHours(12, 0, 0, 0);
  d.setDate(d.getDate() + days);
  return d;
};
export const dateLabel = (days: number, long = false) =>
  dateAt(days).toLocaleDateString(
    "en-GB",
    long
      ? { weekday: "long", day: "numeric", month: "long" }
      : { weekday: "short", day: "numeric", month: "short" },
  );
export const todayLong = dateAt(0).toLocaleDateString("en-GB", {
  weekday: "long",
  day: "numeric",
  month: "long",
});

export const seedEvents: EventItem[] = [
  {
    id: 1,
    title: "Yoga class",
    space: "personal",
    day: 0,
    time: "18:30",
    end: "19:30",
    place: "Studio Hof, Kastanienallee 12",
    repeat: "Every week",
  },
  {
    id: 2,
    title: "Recycling pickup",
    space: "home",
    day: 1,
    allDay: true,
    repeat: "Every week",
  },
  {
    id: 3,
    title: "Plumber visit (kitchen sink)",
    space: "home",
    day: 1,
    time: "08:00",
    end: "10:00",
    place: "Home",
  },
  {
    id: 4,
    title: "Trip planning call",
    space: "trip",
    day: 1,
    time: "20:00",
    end: "20:45",
    repeat: "Every week",
    zone: "19:00 London time",
  },
  {
    id: 5,
    title: "Dentist check-up",
    space: "personal",
    day: 2,
    time: "10:30",
    end: "11:15",
    place: "Zahnarztpraxis Mitte, Torstraße 88",
  },
  {
    id: 6,
    title: "Dinner with the Hoffmanns",
    space: "home",
    day: 3,
    time: "19:00",
    end: "22:00",
    place: "Their place, Lenaustraße 4",
  },
  {
    id: 7,
    title: "Renew passport — appointment window opens",
    space: "personal",
    day: 5,
    allDay: true,
  },
  {
    id: 8,
    title: "Rent due",
    space: "home",
    day: 6,
    allDay: true,
    repeat: "Every month",
  },
  {
    id: 9,
    title: "Yoga class",
    space: "personal",
    day: 7,
    time: "18:30",
    end: "19:30",
    place: "Studio Hof, Kastanienallee 12",
    repeat: "Every week",
  },
  {
    id: 10,
    title: "Recycling pickup",
    space: "home",
    day: 8,
    allDay: true,
    repeat: "Every week",
  },
  {
    id: 11,
    title: "Trip planning call",
    space: "trip",
    day: 8,
    time: "20:30",
    end: "21:15",
    repeat: "Every week",
    zone: "19:30 London time · changed for this date",
  },
  {
    id: 12,
    title: "Yoga class",
    space: "personal",
    day: 14,
    time: "18:30",
    end: "19:30",
    place: "Studio Hof, Kastanienallee 12",
    repeat: "Every week",
  },
];
export const seedTasks: TaskItem[] = [
  {
    id: 1,
    title: "Call Grandma Ifeoma",
    space: "personal",
    day: 0,
    list: "My to-dos",
  },
  {
    id: 2,
    title: "Pick up dry cleaning",
    space: "home",
    day: 0,
    time: "17:00",
    owner: "You",
    reminders: 1,
    list: "Groceries",
  },
  {
    id: 3,
    title: "Eggs, spinach, oat milk",
    space: "home",
    day: 1,
    owner: "Jonas",
    reminders: 1,
    list: "Groceries",
  },
  {
    id: 4,
    title: "Return library books",
    space: "personal",
    day: 3,
    owner: "You",
    reminders: 1,
    list: "My to-dos",
  },
  {
    id: 5,
    title: "Book boiler service",
    space: "home",
    day: 4,
    owner: "Jonas",
    reminders: 1,
    list: "Home admin",
  },
  {
    id: 6,
    title: "Renew home insurance",
    space: "home",
    day: 6,
    time: "09:00",
    owner: "You",
    reminders: 2,
    list: "Home admin",
  },
  {
    id: 7,
    title: "Book airport transfer",
    space: "trip",
    day: 10,
    owner: "Sam",
    reminders: 1,
    list: "Before we go",
  },
  {
    id: 8,
    title: "Reserve Belém Tower tickets",
    space: "trip",
    day: 12,
    owner: "You",
    reminders: 2,
    list: "Before we go",
  },
  {
    id: 9,
    title: "Check passport validity",
    space: "trip",
    list: "Before we go",
  },
  { id: 10, title: "Sunscreen", space: "trip", list: "Packing" },
  {
    id: 11,
    title: "Coffee beans",
    space: "home",
    list: "Groceries",
    done: true,
  },
  {
    id: 12,
    title: "Pay electricity bill",
    space: "home",
    list: "Home admin",
    done: true,
  },
];
export const places: PlaceItem[] = [
  {
    name: "Markthalle Neun",
    category: "Groceries",
    address: "Eisenbahnstraße 42, Berlin",
    note: "Saturday market is the good one.",
    space: "home",
  },
  {
    name: "Apotheke am Kanal",
    category: "Health",
    address: "Paul-Lincke-Ufer 7, Berlin",
    space: "home",
  },
  {
    name: "Café Linde",
    category: "Food & drink",
    address: "Lindenstraße 21, Berlin",
    note: "Best for Sunday breakfast.",
    space: "home",
  },
  {
    name: "Fahrrad Rahn",
    category: "Services",
    address: "Oranienstraße 160, Berlin",
    space: "home",
  },
  {
    name: "Hausarztpraxis Dr. Vogel",
    category: "Health",
    address: "Görlitzer Straße 64, Berlin",
    space: "home",
  },
  {
    name: "Pastéis de Belém",
    category: "Food & drink",
    address: "Rua de Belém 84, Lisbon",
    note: "Go early; the queue moves quickly.",
    space: "trip",
  },
  {
    name: "Time Out Market",
    category: "Food & drink",
    address: "Av. 24 de Julho 49, Lisbon",
    space: "trip",
  },
  {
    name: "Farmácia Belém",
    category: "Health",
    address: "Rua de Belém 2, Lisbon",
    space: "trip",
  },
];
