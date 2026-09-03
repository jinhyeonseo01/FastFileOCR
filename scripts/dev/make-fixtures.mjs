import { mkdir, writeFile } from "node:fs/promises";
const text = (size, x, y, value) =>
  "BT /F1 " +
  size +
  " Tf " +
  x +
  " " +
  y +
  " Td (" +
  value.replace(/[()\\]/g, "\\$&") +
  ") Tj ET";
const invoice = [
  "0.93 0.97 1 rg 42 692 511 100 re f 0.12 0.25 0.38 rg",
  text(25, 66, 753, "BLUE SKY STUDIO"),
  text(12, 67, 723, "WORKSHOP SUPPLIES / INVOICE #1042"),
  "0.2 0.27 0.34 rg",
  text(13, 66, 655, "Prepared for: Riverstone Library"),
  text(12, 66, 628, "Date: September 3, 2026"),
  "0.93 0.97 1 rg 54 566 487 34 re f 0.2 0.27 0.34 rg",
  text(12, 66, 578, "ITEM"),
  text(12, 330, 578, "QUANTITY"),
  text(12, 450, 578, "AMOUNT"),
  text(13, 66, 533, "Sketch notebooks"),
  text(13, 351, 533, "12"),
  text(13, 450, 533, "$72.00"),
  text(13, 66, 488, "Drawing pencils"),
  text(13, 355, 488, "8"),
  text(13, 450, 488, "$24.00"),
  text(13, 66, 443, "Watercolor sets"),
  text(13, 355, 443, "4"),
  text(13, 450, 443, "$64.00"),
  "0.79 0.85 0.9 RG 0.6 w 54 562 m 541 562 l S 54 517 m 541 517 l S 54 472 m 541 472 l S 54 427 m 541 427 l S",
  text(16, 330, 377, "TOTAL   $160.00"),
  text(11, 66, 306, "Thank you for supporting creative learning."),
  text(10, 66, 277, "Please keep this invoice for your records."),
  "0.56 0.65 0.74 rg",
  text(9, 66, 70, "Sample document created for FastFileOCR"),
].join("\n");
const notes = [
  "0.12 0.25 0.38 rg",
  text(25, 66, 750, "WORKSHOP NOTES"),
  text(13, 66, 711, "Creative learning / September 2026"),
  "0.2 0.27 0.34 rg",
  text(17, 66, 646, "Preparation checklist"),
  text(13, 66, 603, "1. Organize materials for each table."),
  text(13, 66, 570, "2. Set up the welcome desk."),
  text(13, 66, 537, "3. Test the projector and speakers."),
  text(17, 66, 465, "Schedule"),
  text(13, 66, 422, "10:00  Welcome and introductions"),
  text(13, 66, 389, "10:30  Guided sketching activity"),
  text(13, 66, 356, "11:30  Share and reflect"),
  text(11, 66, 70, "Sample document created for FastFileOCR"),
].join("\n");
const objects = [
  "<< /Type /Catalog /Pages 2 0 R >>",
  "<< /Type /Pages /Kids [3 0 R 6 0 R] /Count 2 >>",
  "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>",
  "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
  "<< /Length " +
    Buffer.byteLength(invoice) +
    " >>\nstream\n" +
    invoice +
    "\nendstream",
  "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 4 0 R >> >> /Contents 7 0 R >>",
  "<< /Length " +
    Buffer.byteLength(notes) +
    " >>\nstream\n" +
    notes +
    "\nendstream",
];
let pdf = "%PDF-1.4\n";
const offsets = [0];
for (let i = 0; i < objects.length; i++) {
  offsets.push(Buffer.byteLength(pdf));
  pdf += i + 1 + " 0 obj\n" + objects[i] + "\nendobj\n";
}
const xref = Buffer.byteLength(pdf);
pdf += "xref\n0 " + offsets.length + "\n0000000000 65535 f \n";
for (const offset of offsets.slice(1))
  pdf += String(offset).padStart(10, "0") + " 00000 n \n";
pdf +=
  "trailer\n<< /Size " +
  offsets.length +
  " /Root 1 0 R >>\nstartxref\n" +
  xref +
  "\n%%EOF";
await mkdir("docs/assets", { recursive: true });
await writeFile("docs/assets/sample-invoice.pdf", pdf);
