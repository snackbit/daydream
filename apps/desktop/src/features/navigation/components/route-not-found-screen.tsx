import { Link } from '@tanstack/react-router';
import { ArrowLeft, FolderOpen, Home, Video } from 'lucide-react';
import { NarrowPageLayout, PageHeader } from '@/app/PageLayout';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';

export function RouteNotFoundScreen() {
  return (
    <NarrowPageLayout>
      <PageHeader
        title="That page isn't here"
        description="The address may be outdated or typed incorrectly. Your recordings remain in the local Library."
      >
        <Badge className="mt-3" variant="outline">
          Page not found
        </Badge>
      </PageHeader>

      <Card>
        <CardHeader>
          <CardTitle>Find your way back</CardTitle>
          <CardDescription>
            Continue where you left off or open another part of Daydream.
          </CardDescription>
        </CardHeader>
        <CardContent className="flex flex-wrap gap-2">
          <Button asChild className="gap-2">
            <Link to="/">
              <Home aria-hidden="true" className="size-3.5" />
              Home
            </Link>
          </Button>
          <Button asChild className="gap-2" variant="outline">
            <Link to="/library">
              <FolderOpen aria-hidden="true" className="size-3.5" />
              Library
            </Link>
          </Button>
          <Button asChild className="gap-2" variant="outline">
            <Link to="/capture">
              <Video aria-hidden="true" className="size-3.5" />
              Capture
            </Link>
          </Button>
          <Button asChild className="gap-2" size="sm" variant="ghost">
            <Link to="/">
              <ArrowLeft aria-hidden="true" className="size-3.5" />
              Return to Daydream
            </Link>
          </Button>
        </CardContent>
      </Card>
    </NarrowPageLayout>
  );
}
