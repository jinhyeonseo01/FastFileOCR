import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import rehypeRaw from "rehype-raw";
import rehypeSanitize, { defaultSchema } from "rehype-sanitize";
const schema = {
  ...defaultSchema,
  attributes: {
    ...defaultSchema.attributes,
    td: ["rowSpan", "colSpan", "align"],
    th: ["rowSpan", "colSpan", "align"],
  },
};
export default function MarkdownResult({ text }: { text: string }) {
  return (
    <article className="markdown">
      <Markdown
        remarkPlugins={[remarkGfm]}
        rehypePlugins={[rehypeRaw, [rehypeSanitize, schema]]}
        components={{
          img: () => null,
          a: ({ children }) => (
            <span className="document-link">{children}</span>
          ),
        }}
      >
        {text}
      </Markdown>
    </article>
  );
}
