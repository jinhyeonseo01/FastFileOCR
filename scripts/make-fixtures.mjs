import { mkdir, writeFile } from 'node:fs/promises';
await mkdir('outputs/fixtures',{recursive:true});
const stream = [
'BT /F1 26 Tf 70 735 Td (GLYPH DOCUMENT TEST) Tj ET',
'BT /F1 14 Tf 70 695 Td (Invoice No. 2026-0903) Tj ET',
'BT /F1 14 Tf 70 665 Td (Customer: Jin Kim) Tj ET',
'BT /F1 14 Tf 70 620 Td (Item) Tj 240 0 Td (Quantity) Tj 110 0 Td (Amount) Tj ET',
'BT /F1 14 Tf 70 580 Td (Notebook) Tj 240 0 Td (2) Tj 110 0 Td (12000) Tj ET',
'BT /F1 14 Tf 70 540 Td (Pen) Tj 240 0 Td (3) Tj 110 0 Td (3000) Tj ET',
'BT /F1 14 Tf 70 470 Td (Total: 15000 KRW) Tj ET',
'50 610 m 545 610 l S 50 570 m 545 570 l S 50 530 m 545 530 l S',
].join('\n');
const second='BT /F1 26 Tf 70 735 Td (SECOND PAGE) Tj ET\nBT /F1 14 Tf 70 685 Td (Full page OCR preserves page order.) Tj ET';
const objects=[
'<< /Type /Catalog /Pages 2 0 R >>',
'<< /Type /Pages /Kids [3 0 R 6 0 R] /Count 2 >>',
'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 4 0 R >> >> /Contents 5 0 R >>',
'<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>',
'<< /Length '+Buffer.byteLength(stream)+' >>\nstream\n'+stream+'\nendstream',
'<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 4 0 R >> >> /Contents 7 0 R >>',
'<< /Length '+Buffer.byteLength(second)+' >>\nstream\n'+second+'\nendstream'
];
let pdf='%PDF-1.4\n';const offsets=[0];
for(let i=0;i<objects.length;i++){offsets.push(Buffer.byteLength(pdf));pdf+=(i+1)+' 0 obj\n'+objects[i]+'\nendobj\n';}
const xref=Buffer.byteLength(pdf);
pdf+='xref\n0 '+offsets.length+'\n0000000000 65535 f \n';
for(const offset of offsets.slice(1))pdf+=String(offset).padStart(10,'0')+' 00000 n \n';
pdf+='trailer\n<< /Size '+offsets.length+' /Root 1 0 R >>\nstartxref\n'+xref+'\n%%EOF';
await writeFile('outputs/fixtures/invoice-two-pages.pdf',pdf);
