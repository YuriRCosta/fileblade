.pragma library

function hydrationPlan(response) {
  if (!response || typeof response !== "object") {
    return { apply: false, writable: false, text: "", retry: true, warning: "state read returned nothing; keeping state.json read-only" }
  }
  if (response.ok) {
    var quarantined = response.quarantined ? String(response.quarantined) : ""
    return {
      apply: true,
      writable: true,
      text: String(response.text || ""),
      retry: false,
      warning: quarantined ? "unreadable state.json moved to " + quarantined + " (" + String(response.error || "") + ")" : ""
    }
  }
  return {
    apply: false,
    writable: false,
    text: "",
    retry: !response.cancelled,
    warning: "preserving unreadable state.json: " + String(response.error || "read failed")
  }
}
