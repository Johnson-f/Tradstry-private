"use client";

import { cn } from "@/lib/utils";
import { createClient } from "@/lib/supabase/client";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Progress } from "@/components/ui/progress";
import { useRouter } from "next/navigation";
import { useState, useMemo } from "react";
import { Lock, AlertCircle, CheckCircle2, KeyRound, Eye, EyeOff } from "lucide-react";
import { Alert, AlertDescription } from "@/components/ui/alert";

function getPasswordStrength(password: string): { strength: number; label: string; color: string } {
  if (!password) return { strength: 0, label: "", color: "" };
  
  let strength = 0;
  const checks = {
    length: password.length >= 8,
    lowercase: /[a-z]/.test(password),
    uppercase: /[A-Z]/.test(password),
    number: /\d/.test(password),
    special: /[!@#$%^&*(),.?":{}|<>]/.test(password),
  };

  strength = Object.values(checks).filter(Boolean).length;
  
  if (strength <= 2) {
    return { strength: strength * 20, label: "Weak", color: "bg-red-500" };
  } else if (strength <= 4) {
    return { strength: strength * 20, label: "Medium", color: "bg-yellow-500" };
  } else {
    return { strength: strength * 20, label: "Strong", color: "bg-green-500" };
  }
}

export function UpdatePasswordForm({
  className,
  ...props
}: React.ComponentPropsWithoutRef<"div">) {
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [showConfirmPassword, setShowConfirmPassword] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const router = useRouter();

  const passwordStrength = useMemo(() => getPasswordStrength(password), [password]);

  const passwordChecks = useMemo(() => {
    return {
      length: password.length >= 8,
      lowercase: /[a-z]/.test(password),
      uppercase: /[A-Z]/.test(password),
      number: /\d/.test(password),
      special: /[!@#$%^&*(),.?":{}|<>]/.test(password),
    };
  }, [password]);

  const handleUpdatePassword = async (e: React.FormEvent) => {
    e.preventDefault();
    const supabase = createClient();
    setIsLoading(true);
    setError(null);

    if (password !== confirmPassword) {
      setError("Passwords do not match");
      setIsLoading(false);
      return;
    }

    if (password.length < 8) {
      setError("Password must be at least 8 characters long");
      setIsLoading(false);
      return;
    }

    try {
      const { error } = await supabase.auth.updateUser({ password });
      if (error) throw error;
      router.push("/app");
    } catch (error: unknown) {
      setError(error instanceof Error ? error.message : "An error occurred");
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className={cn("flex flex-col gap-6", className)} {...props}>
      <Card className="border-border/50 shadow-lg">
        <CardHeader className="space-y-3 text-center pb-6">
          <div className="mx-auto mb-2 flex size-12 items-center justify-center rounded-full bg-primary/10">
            <KeyRound className="size-6 text-primary" />
          </div>
          <CardTitle className="text-3xl font-bold tracking-tight">Set new password</CardTitle>
          <CardDescription className="text-base">
            Choose a strong password to secure your account
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          <form onSubmit={handleUpdatePassword} className="space-y-5">
            {error && (
              <Alert variant="destructive" className="border-destructive/50">
                <AlertCircle className="size-4" />
                <AlertDescription>{error}</AlertDescription>
              </Alert>
            )}

            <div className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="password" className="text-sm font-medium">
                  New password
                </Label>
                <div className="relative">
                  <Lock className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
                <Input
                  id="password"
                    type={showPassword ? "text" : "password"}
                    placeholder="Create a strong password"
                  required
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                    className="pl-10 pr-10 h-11"
                    disabled={isLoading}
                  />
                  <button
                    type="button"
                    onClick={() => setShowPassword(!showPassword)}
                    className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground transition-colors"
                    tabIndex={-1}
                  >
                    {showPassword ? (
                      <EyeOff className="size-4" />
                    ) : (
                      <Eye className="size-4" />
                    )}
                  </button>
                </div>
                {password && (
                  <div className="space-y-2 pt-2">
                    <div className="flex items-center justify-between text-xs">
                      <span className="text-muted-foreground">Password strength</span>
                      <span className={cn(
                        "font-medium",
                        passwordStrength.label === "Weak" && "text-red-500",
                        passwordStrength.label === "Medium" && "text-yellow-500",
                        passwordStrength.label === "Strong" && "text-green-500"
                      )}>
                        {passwordStrength.label}
                      </span>
                    </div>
                    <Progress value={passwordStrength.strength} className="h-1.5" />
                    <div className="grid grid-cols-2 gap-2 text-xs">
                      <div className={cn("flex items-center gap-1.5", passwordChecks.length && "text-green-600 dark:text-green-400")}>
                        {passwordChecks.length ? (
                          <CheckCircle2 className="size-3" />
                        ) : (
                          <div className="size-3 rounded-full border border-muted-foreground/30" />
                        )}
                        <span className={cn(!passwordChecks.length && "text-muted-foreground")}>
                          8+ characters
                        </span>
                      </div>
                      <div className={cn("flex items-center gap-1.5", passwordChecks.lowercase && "text-green-600 dark:text-green-400")}>
                        {passwordChecks.lowercase ? (
                          <CheckCircle2 className="size-3" />
                        ) : (
                          <div className="size-3 rounded-full border border-muted-foreground/30" />
                        )}
                        <span className={cn(!passwordChecks.lowercase && "text-muted-foreground")}>
                          Lowercase
                        </span>
                      </div>
                      <div className={cn("flex items-center gap-1.5", passwordChecks.uppercase && "text-green-600 dark:text-green-400")}>
                        {passwordChecks.uppercase ? (
                          <CheckCircle2 className="size-3" />
                        ) : (
                          <div className="size-3 rounded-full border border-muted-foreground/30" />
                        )}
                        <span className={cn(!passwordChecks.uppercase && "text-muted-foreground")}>
                          Uppercase
                        </span>
                      </div>
                      <div className={cn("flex items-center gap-1.5", passwordChecks.number && "text-green-600 dark:text-green-400")}>
                        {passwordChecks.number ? (
                          <CheckCircle2 className="size-3" />
                        ) : (
                          <div className="size-3 rounded-full border border-muted-foreground/30" />
                        )}
                        <span className={cn(!passwordChecks.number && "text-muted-foreground")}>
                          Number
                        </span>
                      </div>
                    </div>
                  </div>
                )}
              </div>

              <div className="space-y-2">
                <Label htmlFor="confirm-password" className="text-sm font-medium">
                  Confirm password
                </Label>
                <div className="relative">
                  <Lock className="absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
                  <Input
                    id="confirm-password"
                    type={showConfirmPassword ? "text" : "password"}
                    placeholder="Confirm your password"
                    required
                    value={confirmPassword}
                    onChange={(e) => setConfirmPassword(e.target.value)}
                    className={cn(
                      "pl-10 pr-10 h-11",
                      confirmPassword && password !== confirmPassword && "border-destructive focus-visible:border-destructive"
                    )}
                    disabled={isLoading}
                  />
                  <button
                    type="button"
                    onClick={() => setShowConfirmPassword(!showConfirmPassword)}
                    className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground transition-colors"
                    tabIndex={-1}
                  >
                    {showConfirmPassword ? (
                      <EyeOff className="size-4" />
                    ) : (
                      <Eye className="size-4" />
                    )}
                  </button>
                </div>
                {confirmPassword && password !== confirmPassword && (
                  <p className="text-xs text-destructive flex items-center gap-1">
                    <AlertCircle className="size-3" />
                    Passwords do not match
                  </p>
                )}
                {confirmPassword && password === confirmPassword && password && (
                  <p className="text-xs text-green-600 dark:text-green-400 flex items-center gap-1">
                    <CheckCircle2 className="size-3" />
                    Passwords match
                  </p>
                )}
              </div>
            </div>

            <Button 
              type="submit" 
              className="w-full h-11 text-base font-medium shadow-sm" 
              // @ts-expect-error - will fix later (i may never, inasmuch as the code works, who cares?)
              disabled={isLoading || (password && passwordStrength.strength < 60)}
            >
              {isLoading ? (
                <>
                  <span className="mr-2 inline-block size-4 animate-spin rounded-full border-2 border-current border-t-transparent" />
                  Updating password...
                </>
              ) : (
                "Update password"
              )}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
