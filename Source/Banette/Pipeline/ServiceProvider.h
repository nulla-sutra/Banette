// Copyright 2019-Present tarnishablec. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "Core/Service.h"

namespace Banette::Pipeline
{
	using namespace Banette::Core;

	template <CService T>
	struct TServiceProvider
	{
		virtual ~TServiceProvider() = default;

		static UE5Coro::TCoroutine<TSharedPtr<T>> BuildService(bool& bSuccess)
		{
			static_assert(sizeof(T) == 0,
			              "Banette::Pipeline::GetServiceImpl(TServiceTag<T>) is not specialized "
			              "for this service type T. Please provide an overload in some header.");
			return nullptr;
		}

		static UE5Coro::TCoroutine<TSharedPtr<T>> GetService()
		{
			static TSharedPtr<T> Service = nullptr;
			static bool bSuccess = true;

			if (!Service.IsValid() || !bSuccess)
			{
				Service = co_await BuildService(bSuccess);
				co_return Service;
			}
			co_return Service;
		}
	};
}

#define BANETTE_SERVICE_PROVIDER(T) \
static UE5Coro::TCoroutine<TSharedPtr<T>> GetService() \
{ \
static TSharedPtr<T> Service = nullptr; \
static bool bSuccess = true; \
\
if (!Service.IsValid() || !bSuccess) \
{ \
Service = co_await BuildService(bSuccess); \
co_return Service; \
} \
co_return Service; \
} \
static UE5Coro::TCoroutine<TSharedPtr<T>> BuildService(bool& bSuccess)
