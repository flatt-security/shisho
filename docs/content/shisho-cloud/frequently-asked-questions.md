---
title: 'Frequently Asked Questions'
metaTitle: 'Frequently Asked Questions - Shisho Cloud'
metaDescription: 'This page shows frequently asked questions.'
---

### Sign Up

#### I do not have any SSO accounts. What should I do?

At this stage, we do not provide email/password-based sign-up and the other SSO options. For the registration, [Shisho Cloud](https://shisho.dev/) needs to integrate with at least one of the repository managers. This way gives you centrally to manage both the account and integration configuration. If we continuously receive the request, we consider them but please consider creating the account of the repository managers.

#### Can I sign up with multiple SSOs?

Yes, but [Shisho Cloud](https://shisho.dev/) treats your multiple accounts as different accounts even though you work with the same repositories in repository managers.

### Repository connection

#### Can I link with repositories of multiple repository managers such as GitHub and Bitbucket?

Yes,  you are able to link with repositories of multiple repository managers. We know some enterprises and small businesses separately own repositories due to the company policy. You do not need to sign up with each of them, please simply log in by the current SSO and connect with the other repository managers.

#### Can I connect Shisho Cloud with my own Git server?

Unfortunately, we do not support the own Git servers at this stage but we of course consider that. If you seriously want, please send feedback. We might prioritize it higher.

#### Can I connect Shisho Cloud with other services such as Azure Repos?

We support [GitHub](https://github.com/), [GitLab](https://about.gitlab.com/), and [BitBucket](https://bitbucket.org/product) at the moment but we consider expanding the other services. If you want, please send feedback regarding the request. We might change task priorities.

#### I want to test Shisho Cloud but I have not had Terraform code yet. Is it possible?

Please consider trying a test repository. You might be able to understand what IaC is and why the security is significant for it. As you know, IaC such as [Terrafrom](https://www.terraform.io/) is useful and powerful to create, update and destroy cloud resources. We are happy to support the new journey of secured cloud resource management.

### Test repository "[flatt-security/tfgoat-aws](https://github.com/flatt-security/tfgoat-aws)"

#### What is a test repository?

The test repository, "[flatt-security/tfgoat-aws](https://github.com/flatt-security/tfgoat-aws)" is our vulnerable-by-design [Terrafrom](https://www.terraform.io/) repository for testing purposes. Why we created is that some clients want us to demonstrate [Shisho Cloud](https://shisho.dev/) without connecting with their repositories. We are pretty sure that it is enough to assess the quality and performance of [Shisho Cloud](https://shisho.dev/).

#### So, will I have any risks with the test repository?

No worries, friends. You should not have any troubles with your cloud services and of course, repositories as well. The [Terrafrom](https://www.terraform.io/) code misconfigures [AWS](https://aws.amazon.com/) resources and policies for dummy resources on purpose but it will never pose critical incidents for your existing resources.

#### Is it safe if I keep the test repository?

Of course, yes. You should not have any troubles with your cloud services and repositories. However, if you want to delete it, you can remove it from your repository managers, [GitHub](https://github.com/), [GitLab](https://about.gitlab.com/), and [BitBucket](https://bitbucket.org/product). Moreover, you can delete the repository integration OR your account itself of [Shisho Cloud](https://shisho.dev/).

### Shisho GitHub App

#### What is Shisho GitHub App?

Shisho GitHub App is one of the official "GitHub Apps" and our [GitHub](https://github.com/) repository integration requires it. Shisho GitHub Apps can be installed directly on organizations and user accounts and granted access to specific repositories via GitHub Apps.

#### I do not want to install Shisho GitHub App on my PC. Is it OK?

Some people are confused "GitHub Apps" as a desktop application that you need to install for your machines. This is an extension for your [GitHub](https://github.com/) account for the seamless integration and your manageable identities and assets.

#### I cannot install Shisho GitHub App for repositories managed by my employer. Why?

If you want to use [Shisho Cloud](https://shisho.dev/) with your workplace repositories, you might need to ask your repository administrator to install the app at the organization level. If it is difficult for testing purposes, please consider installing it on your private account.

####  Is it possible to uninstall Shisho GitHub App?

Yes, you can uninstall Shisho GitHub App from [GitHub](https://github.com/) Profile menu. Please go to Settings -> Applications / Integrations section.

### Shisho Cloud account

####  Is it possible to delete Shisho Cloud?

Yes, you can delete the [Shisho Cloud](https://shisho.dev/) account by yourself but we might cry :( We remove SSO details for login and discard all your work.