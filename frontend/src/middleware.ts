import { clerkMiddleware, createRouteMatcher } from "@clerk/nextjs/server";
import { NextResponse } from "next/server";

const isPublicRoute = createRouteMatcher(["/", "/sign-in(.*)", "/sign-up(.*)"]);

export default clerkMiddleware(async (auth, req) => {
	const { userId } = await auth();

	// Redirect signed-in users away from public routes to dashboard
	if (userId && isPublicRoute(req)) {
		return NextResponse.redirect(new URL("/dashboard", req.url));
	}

	// Protect all non-public routes
	if (!userId && !isPublicRoute(req)) {
		return NextResponse.redirect(new URL("/sign-in", req.url));
	}
});

export const config = {
	matcher: [
		"/((?!_next|[^?]*\\.(?:html?|css|js(?!on)|jpe?g|webp|png|gif|svg|ttf|woff2?|ico|csv|docx?|xlsx?|zip|webmanifest)).*)",
		"/(api|trpc)(.*)",
	],
};
