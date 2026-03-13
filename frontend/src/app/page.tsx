"use client";

import { SignInButton, SignUpButton, UserButton, useAuth } from "@clerk/nextjs";

export default function Home() {
	const { isSignedIn, isLoaded } = useAuth();

	if (!isLoaded) return null;

	return (
		<div className="flex min-h-screen flex-col items-center justify-center gap-8 bg-zinc-50 font-sans dark:bg-black">
			<h1 className="text-4xl font-semibold tracking-tight text-black dark:text-zinc-50">
				Tradstry
			</h1>
			{isSignedIn ? (
				<UserButton />
			) : (
				<div className="flex gap-4">
					<SignInButton>
						<button className="flex h-12 items-center justify-center rounded-full bg-foreground px-8 text-base font-medium text-background transition-colors hover:bg-[#383838] dark:hover:bg-[#ccc]">
							Sign In
						</button>
					</SignInButton>
					<SignUpButton>
						<button className="flex h-12 items-center justify-center rounded-full border border-solid border-black/[.08] px-8 text-base font-medium transition-colors hover:border-transparent hover:bg-black/[.04] dark:border-white/[.145] dark:hover:bg-[#1a1a1a]">
							Sign Up
						</button>
					</SignUpButton>
				</div>
			)}
		</div>
	);
}
