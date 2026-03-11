import { SignUpForm } from "@/components/sign-up-form";
import Link from "next/link";

export default function Page() {
  return (
    <div className="relative flex min-h-svh w-full items-center justify-center p-6 md:p-10">
      {/* Background pattern */}
      <div 
        className="absolute inset-0 -z-10 opacity-[0.02]" 
        style={{
          backgroundImage: 'radial-gradient(circle, rgba(0,0,0,0.1) 1px, transparent 1px)',
          backgroundSize: '20px 20px'
        }} 
      />
      <div className="absolute inset-0 -z-10 bg-gradient-to-b from-background via-background to-muted/20" />
      
      {/* Logo/Brand link */}
      <Link 
        href="/" 
        className="absolute top-6 left-6 text-xl font-bold tracking-tight hover:opacity-80 transition-opacity"
      >
        TRADSTRY
      </Link>

      <div className="w-full max-w-md">
        <SignUpForm />
      </div>
    </div>
  );
}
