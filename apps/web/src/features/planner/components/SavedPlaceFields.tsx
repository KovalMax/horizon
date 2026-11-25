import { useEffect, useRef, useState } from "react";
import { Button } from "../../../shared/ui";
import {
  PlacesApiError,
  previewPlaceImport,
  type PlaceImportPreview,
} from "../placesApi";

function importError(error: unknown) {
  switch (error instanceof PlacesApiError ? error.status : 0) {
    case 400:
      return "Paste a supported Google Maps place link.";
    case 422:
      return "This link doesn’t contain importable place details. Enter them manually.";
    case 403:
    case 404:
      return "You can’t import a place into this Space.";
    case 429:
      return "Too many imports. Wait a minute and try again.";
    default:
      return "We couldn’t open this link. Try again or enter the details manually.";
  }
}

export function SavedPlaceFields({
  accessToken,
  spaceId,
}: {
  accessToken: string;
  spaceId: string;
}) {
  const [link, setLink] = useState("");
  const [name, setName] = useState("");
  const [address, setAddress] = useState("");
  const [note, setNote] = useState("");
  const [hours, setHours] = useState("");
  const [cuisine, setCuisine] = useState("");
  const [website, setWebsite] = useState("");
  const [noteError, setNoteError] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);
  const [preview, setPreview] = useState<PlaceImportPreview | null>(null);
  const pending = useRef<AbortController | null>(null);

  useEffect(() => () => pending.current?.abort(), []);

  async function importDetails() {
    if (!link.trim() || pending.current) return;
    const request = new AbortController();
    pending.current = request;
    setLoading(true);
    setError("");
    setPreview(null);
    try {
      const result = await previewPlaceImport(
        accessToken,
        spaceId,
        link.trim(),
        request.signal,
      );
      if (request.signal.aborted) return;
      setPreview(result);
      setName((current) =>
        current.trim() ? current : (result.name ?? current),
      );
      setAddress((current) =>
        current.trim() ? current : (result.address ?? current),
      );
    } catch (error) {
      if (!request.signal.aborted) setError(importError(error));
    } finally {
      if (pending.current === request) {
        pending.current = null;
        setLoading(false);
      }
    }
  }

  function addToNote() {
    const lines = [
      hours.trim() && `Hours: ${hours.trim()}`,
      cuisine.trim() && `Cuisine: ${cuisine.trim()}`,
      website.trim() && `Website: ${website.trim()}`,
    ].filter(Boolean);
    const next = [note.trimEnd(), lines.join("\n")]
      .filter(Boolean)
      .join("\n\n");
    if ([...next].length > 2000) {
      setNoteError("Shorten the note or the details before adding them.");
      return;
    }
    setNote(next);
    setHours("");
    setCuisine("");
    setWebsite("");
    setNoteError("");
  }

  return (
    <>
      <details className="place-extra-fields">
        <summary>Import from Google Maps</summary>
        <label className="form-field">
          Google Maps link
          <input
            type="text"
            inputMode="url"
            autoComplete="off"
            maxLength={4096}
            placeholder="https://maps.app.goo.gl/…"
            value={link}
            onChange={(event) => {
              pending.current?.abort();
              pending.current = null;
              setLoading(false);
              setError("");
              setPreview(null);
              setLink(event.target.value);
            }}
            onKeyDown={(event) => {
              if (event.key === "Enter") {
                event.preventDefault();
                void importDetails();
              }
            }}
          />
        </label>
        <Button
          variant="outline"
          disabled={!link.trim() || loading}
          onClick={() => void importDetails()}
        >
          Import details
        </Button>
        {loading && <p role="status">Importing place details…</p>}
        {error && <p role="alert">{error}</p>}
        {preview && (
          <p role="status">
            {preview.name && preview.address
              ? "Review the imported name and address before saving."
              : preview.name
                ? "Review the imported name and enter the address if missing."
                : "Review the imported address and enter the name if missing."}
          </p>
        )}
      </details>
      <label className="form-field">
        Place name
        <input
          name="title"
          required
          maxLength={120}
          value={name}
          onChange={(event) => setName(event.target.value)}
        />
      </label>
      {preview?.name && preview.name !== name && (
        <Button variant="outline" onClick={() => setName(preview.name!)}>
          Use imported name: {preview.name}
        </Button>
      )}
      <label className="form-field">
        Address
        <input
          name="address"
          placeholder="Street, city"
          required
          maxLength={300}
          value={address}
          onChange={(event) => setAddress(event.target.value)}
        />
      </label>
      {preview?.address && preview.address !== address && (
        <Button variant="outline" onClick={() => setAddress(preview.address!)}>
          Use imported address: {preview.address}
        </Button>
      )}
      <label className="form-field">
        Category
        <select name="category">
          <option>Food &amp; drink</option>
          <option>Groceries</option>
          <option>Health</option>
          <option>Services</option>
        </select>
      </label>
      <label className="form-field">
        Note (optional)
        <textarea
          name="note"
          rows={3}
          maxLength={2000}
          placeholder="A useful detail for everyone"
          value={note}
          onChange={(event) => setNote(event.target.value)}
        />
      </label>
      <details className="place-extra-fields">
        <summary>Add opening hours, cuisine, or website</summary>
        <p>
          Enter the details you want to keep, for example from the place’s own
          website.
        </p>
        <label className="form-field">
          Opening hours
          <input
            value={hours}
            maxLength={500}
            onChange={(event) => setHours(event.target.value)}
          />
        </label>
        <label className="form-field">
          Cuisine
          <input
            value={cuisine}
            maxLength={120}
            onChange={(event) => setCuisine(event.target.value)}
          />
        </label>
        <label className="form-field">
          Website
          <input
            inputMode="url"
            value={website}
            maxLength={500}
            onChange={(event) => setWebsite(event.target.value)}
          />
        </label>
        <Button
          variant="outline"
          disabled={![hours, cuisine, website].some((value) => value.trim())}
          onClick={addToNote}
        >
          Add to note
        </Button>
        {noteError && <p role="alert">{noteError}</p>}
      </details>
    </>
  );
}
