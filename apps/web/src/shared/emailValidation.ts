const domainTypos = new Map([
  ["gmial.com", "gmail.com"],
  ["gamil.com", "gmail.com"],
  ["gmai.com", "gmail.com"],
  ["gmail.con", "gmail.com"],
  ["hotmial.com", "hotmail.com"],
  ["outlok.com", "outlook.com"],
  ["yaho.com", "yahoo.com"],
]);

export async function readProblemCode(response: Response): Promise<string> {
  const problem = await response.json().catch(() => null);
  return typeof problem?.code === "string" ? problem.code : "";
}

export function emailValidationMessage(code: string, email: string) {
  switch (code) {
    case "invalid_email":
    case "invalid_invitation":
      return "Enter a valid email address.";
    case "email_domain_unavailable":
      return "We couldn’t check this email address. Please try again.";
    case "email_domain_no_mail": {
      const domain = email.trim().split("@").at(-1)?.toLowerCase();
      const suggestion = domain ? domainTypos.get(domain) : undefined;
      return suggestion
        ? `This email domain doesn’t receive mail. Did you mean ${suggestion}?`
        : "This email domain doesn’t receive mail. Check the part after @.";
    }
    default:
      return undefined;
  }
}
