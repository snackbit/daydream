export interface PlaceholderScreenProps {
  summary: string;
  title: string;
}

export function PlaceholderScreen({ summary, title }: PlaceholderScreenProps) {
  return (
    <section className="mx-auto flex max-w-5xl flex-col gap-3 p-6">
      <h1 className="text-2xl font-semibold tracking-normal">{title}</h1>
      <p className="text-muted-foreground max-w-3xl text-sm leading-6">
        {summary}
      </p>
      <div className="bg-card text-muted-foreground border p-4 text-sm">
        Placeholder only. The scaffold keeps the navigation surface in place
        without pretending this feature is implemented.
      </div>
    </section>
  );
}
