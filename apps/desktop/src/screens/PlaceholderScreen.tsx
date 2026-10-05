import { Link } from '@tanstack/react-router';
import { ArrowRight, Construction, FolderOpen, Video } from 'lucide-react';
import { PageHeader, PageLayout } from '@/app/PageLayout';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';

export interface PlaceholderScreenProps {
  summary: string;
  title: string;
}

export function PlaceholderScreen({ summary, title }: PlaceholderScreenProps) {
  return (
    <PageLayout>
      <PageHeader title={title} description={summary}>
        <Badge className="mt-3 gap-1.5" variant="outline">
          <Construction aria-hidden="true" className="size-3" />
          In development
        </Badge>
      </PageHeader>

      <Card className="max-w-3xl">
        <CardHeader>
          <CardTitle>This screen is not available yet</CardTitle>
          <CardDescription className="max-w-2xl leading-5">
            Daydream is being built in small, reviewable steps. This part of the
            workspace does not have working data or actions yet.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-wrap gap-2">
          <Button asChild className="gap-2">
            <Link to="/capture">
              <Video aria-hidden="true" className="size-3.5" />
              Open Capture
              <ArrowRight aria-hidden="true" className="size-3.5" />
            </Link>
          </Button>
          <Button asChild className="gap-2" variant="outline">
            <Link to="/library">
              <FolderOpen aria-hidden="true" className="size-3.5" />
              Browse Library
            </Link>
          </Button>
        </CardContent>
      </Card>
    </PageLayout>
  );
}
