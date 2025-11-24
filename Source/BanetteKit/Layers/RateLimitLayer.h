#pragma once

#include "CoreMinimal.h"
#include "Banette.h"
#include "UE5Coro.h"

namespace Banette::Kit
{
	using namespace Banette::Core;

	struct FRateLimitConfig
	{
		// Tokens added per second (Rate)
		double TokensPerSecond = 5.0;
		// Bucket capacity (Burst)
		double MaxTokens = 10.0;
		// If no token is available, wait asynchronously? (false returns an error immediately)
		bool bWaitForToken = true;
	};

	/**
	 * Rate limiting layer.
	 * The layer itself only holds configuration. Runtime state (TokenBucket) lives in the created Service.
	 * Therefore, each wrapped Service has its own independent rate limit bucket.
	 */
	template <CService ServiceT>
	class TRateLimitLayer : public TLayer<ServiceT, ServiceT>
	{
	public:
		explicit TRateLimitLayer(const FRateLimitConfig& InConfig)
			: Config(InConfig)
		{
		}

		// Wrap is const because it only reads configuration and does not modify the layer itself
		virtual TSharedRef<ServiceT> Wrap(TSharedRef<ServiceT> Inner) const override
		{
			// Create a new Service inside Wrap; state is initialized in the Service constructor
			return MakeShared<FRateLimitService>(Inner, Config);
		}

	private:
		FRateLimitConfig Config;

		// =========================================================
		// Service implementation (holds runtime state)
		// =========================================================
		class FRateLimitService : public ServiceT
		{
		public:
			FRateLimitService(TSharedRef<ServiceT> InInner, const FRateLimitConfig& InConfig)
				: InnerService(InInner)
				  , Config(InConfig)
				  // Initialize state: usually start with a full bucket
				  , CurrentTokens(InConfig.MaxTokens)
				  , LastRefillTime(FPlatformTime::Seconds())
			{
			}

			virtual UE5Coro::TCoroutine<TResult<typename ServiceT::ResponseType>> Call(
				const ServiceT::RequestType& Request) override
			{
				// Try to acquire a token
				if (Config.bWaitForToken)
				{
					// If waiting is required, loop until a token is acquired
					co_await WaitForToken();
				}
				else
				{
					// Non-blocking mode: try to acquire; on failure return an error immediately
					if (!TryAcquireToken())
					{
						// A proper error code should be defined; temporarily return an empty TResult
						co_return TResult<typename ServiceT::ResponseType>();
					}
				}

				// Token acquired; proceed with the call
				co_return InnerService->Call(Request);
			}

		private:
			TSharedRef<ServiceT> InnerService;
			const FRateLimitConfig Config;

			// --- Runtime state (State) ---
			// No TSharedPtr is needed because the state belongs to this Service instance
			// Protected by a CriticalSection to prevent races when the same Service instance is called from multiple threads
			FCriticalSection Mutex;
			double CurrentTokens;
			double LastRefillTime;

			// Attempt to acquire a token (non-blocking)
			bool TryAcquireToken()
			{
				FScopeLock Lock(&Mutex);
				RefillTokens();

				if (CurrentTokens >= 1.0)
				{
					CurrentTokens -= 1.0;
					return true;
				}
				return false;
			}

			// Asynchronously wait for a token
			UE5Coro::TCoroutine<> WaitForToken()
			{
				while (true)
				{
					double WaitTime;
					{
						FScopeLock Lock(&Mutex);
						RefillTokens();

						if (CurrentTokens >= 1.0)
						{
							CurrentTokens -= 1.0;
							co_return; // Acquired successfully; exit
						}
						
						// Compute the required wait time
						const double Missing = 1.0 - CurrentTokens;
						WaitTime = Missing / Config.TokensPerSecond;
					}
					
					if (WaitTime > 0.0)
					{
						// Suspend the coroutine to wait
						co_await UE5Coro::Latent::Seconds(WaitTime);
					}
				}
			}

			// Internal helper: refill tokens based on elapsed time
			void RefillTokens()
			{
				const double Now = FPlatformTime::Seconds();
				const double Elapsed = Now - LastRefillTime;

				if (Elapsed > 0)
				{
					const double NewTokens = Elapsed * Config.TokensPerSecond;
					CurrentTokens = FMath::Min(Config.MaxTokens, CurrentTokens + NewTokens);
					LastRefillTime = Now;
				}
			}
		};
	};
}
