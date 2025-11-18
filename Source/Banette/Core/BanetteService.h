// Copyright 2019-Present tarnishablec. All Rights Reserved.

#pragma once

#include "CoreMinimal.h"
#include "BanetteError.h"
#include "UE5Coro.h"

namespace Banette::Core
{
	template <typename RequestT, typename ResponseT, Error::CUnifiedError ErrorT = UE::UnifiedError::FError>
	class BANETTE_API TService : public TSharedFromThis<TService<RequestT, ResponseT, ErrorT>>
	{
		using RequestType = RequestT;
		using ResponseType = ResponseT;
		using ErrorType = ErrorT;

	public:
		virtual ~TService() = default;

		virtual UE5Coro::TCoroutine<TResult<ResponseT, ErrorT>> Call(const RequestT& Request) = 0;
	};


	template <typename RequestT, typename ResponseT, typename ErrorT= UE::UnifiedError::FError>
	using TServiceRef = TSharedRef<TService<RequestT, ResponseT, ErrorT>>;


	template <typename S>
	concept CService =
		std::is_base_of_v<
			TService<typename S::RequestType, typename S::ResponseType, typename S::ErrorType>,
			S>;
}
