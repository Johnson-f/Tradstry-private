'use client';

import React from 'react';
import { Card, CardFooter, CardHeader, CardTitle, CardDescription } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { ChevronDown } from 'lucide-react';

interface PlaybookProps {
  onAdd?: () => void;
  className?: string;
}

export default function Playbook({ onAdd, className }: PlaybookProps) {
  return (
    <Card className={className}>
      <CardHeader className="text-center">
        <CardTitle>Playbook</CardTitle>
        <CardDescription>
          Add your playbook to track what works and repeat your best setups.
        </CardDescription>
      </CardHeader>
      <CardFooter className="flex justify-center">
        <Button onClick={onAdd} variant="default">
          Add Playbook
          <ChevronDown className="ml-2 h-4 w-4" />
        </Button>
      </CardFooter>
    </Card>
  );
}


