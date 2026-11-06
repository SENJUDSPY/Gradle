import { fetch } from '@tauri-apps/plugin-http'

export type GradleNewsArticle = {
	id: string
	title: string
	summary: string
	/** API-provided image source, preserved unchanged; omitted sources become null. */
	image_url: string | null
	url: string | null
	published_at: string
}

export const GRADLE_NEWS_API = `${import.meta.env.XORISON_API_URL}public/product/gradle/news`

export class GradleNewsError extends Error {
	constructor(
		message: string,
		public readonly httpStatus: number,
	) {
		super(message)
		this.name = 'GradleNewsError'
	}
}

type GradleNewsResponseArticle = Omit<GradleNewsArticle, 'image_url'> & {
	image_url?: string | null
}

function isNewsArticle(value: unknown): value is GradleNewsResponseArticle {
	if (typeof value !== 'object' || value === null) {
		return false
	}

	const article = value as Record<string, unknown>
	return (
		typeof article.id === 'string' &&
		typeof article.title === 'string' &&
		typeof article.summary === 'string' &&
		(!Object.hasOwn(article, 'image_url') ||
			article.image_url === null ||
			typeof article.image_url === 'string') &&
		(article.url === null || typeof article.url === 'string') &&
		typeof article.published_at === 'string' &&
		Number.isFinite(Date.parse(article.published_at))
	)
}

/** Fetches the complete news list and sorts newest first client-side, without filtering or deduplication. */
export async function fetchGradleNews(
	signal?: AbortSignal,
): Promise<GradleNewsArticle[]> {
	const response = await fetch(GRADLE_NEWS_API, {
		method: 'GET',
		headers: { Accept: 'application/json' },
		signal,
	})

	if (!response.ok) {
		throw new GradleNewsError(
			`Gradle news request failed (HTTP ${response.status})`,
			response.status,
		)
	}

	let data: unknown
	try {
		data = await response.json()
	} catch (error) {
		if (signal?.aborted || !(error instanceof SyntaxError)) {
			throw error
		}
		throw new GradleNewsError('Gradle news response is not valid JSON', response.status)
	}

	if (!Array.isArray(data) || !data.every(isNewsArticle)) {
		throw new GradleNewsError('Gradle news response is invalid', response.status)
	}

	return data
		.map(({ id, title, summary, image_url, url, published_at }) => ({
			id,
			title,
			summary,
			image_url: image_url ?? null,
			url,
			published_at,
		}))
		.sort((a, b) => Date.parse(b.published_at) - Date.parse(a.published_at))
}
